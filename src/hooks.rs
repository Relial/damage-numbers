use anyhow::Result;
use bunny_plugin::{
    bunny_ui::vec2,
    hook_builder::{NoCbHookBuilder, NoCbHookPoint},
};
use glam::vec3;
use ilhook::x86::{HookType, Registers};
use mhfz_structs::{AttackEffectInfo, Entity, HitzoneInfo};
use tracing::debug;

use crate::{
    address::Addresses,
    config::ColorSource,
    damage::{DamageInstance, DamageSettings},
    plugin::STATE,
};

pub unsafe extern "C" fn on_quest_update() {
    let state = unsafe { STATE.get_unchecked_mut() };
    let config = &state.config;
    if !config.ice_age_show {
        return;
    }
    let Some(player_info) = state.structs.player_info() else {
        return;
    };
    let Some(monsters) = state.structs.monsters() else {
        return;
    };

    for monster in monsters.filter(|m| m.exists() && m.area() == player_info.area()) {
        if config.hide_damage_on_small_monsters && !monster.is_large() {
            continue;
        }

        let damage_before_defense = monster.ice_age_damage();
        if damage_before_defense < 1 {
            continue;
        }
        let defense = monster.defense_multiplier();
        let damage = ((damage_before_defense as f32 * defense) as i16).max(1);
        let position = monster.pos() + vec3(0.0, 200.0, 0.0);
        let settings = DamageSettings::default()
            .with_color(config.ice_age.color)
            .with_position_offset(state.ice_age_offset.next());
        let tick = DamageInstance::new(damage, position, &state.num_formatter, settings);
        state.ice_age.push(tick);
    }
}

unsafe extern "cdecl" fn on_hit_finalized(reg: *mut Registers, _: usize) {
    unsafe {
        let state = STATE.get_unchecked_mut();
        let config = &state.config;

        let Some(player_info) = state.structs.player_info() else {
            return;
        };
        let monster = state.structs.monster_from_ptr((*reg).esi as *mut u8);
        if monster.area() != player_info.area() {
            return;
        }
        if config.hide_damage_on_small_monsters && !monster.is_large() {
            return;
        }

        let attack = AttackEffectInfo::new((*reg).edi as *mut u8);
        let attacker_addr = attack.attacker_addr() as usize;
        if config.own_attacks_only
            && attacker_addr
                != state
                    .structs
                    .own_player_from_info(player_info)
                    .inner()
                    .addr()
        {
            return;
        }
        if config.hide_halk_attacks && state.structs.is_address_part_of_halk_structs(attacker_addr)
        {
            return;
        }

        let damage_before_defense = ((*reg).eax & 0xFFFF) as i16;
        let defense = monster.defense_multiplier();
        let hit_damage = (damage_before_defense as f32 * defense) as i16;
        let max_health = monster.max_health();
        let portion_of_max = hit_damage as f32 / max_health as f32;
        let range = config.get_range(portion_of_max);
        let mut hit_position = attack.hit_position();

        // MS pin finisher (and possibly other attacks) have a nonsensical hit position in the void, so sanity check here
        // Ravi's position is in the middle of the arena and the edges are ~6000 units from there
        // Hopefully there isn't anything I'm not thinking of that's further
        // MS pin finisher seems to hit 12,000+ units away but varies by monster/area
        let monster_position = monster.pos();
        const MAX_DISTANCE_SQUARED: f32 = 8000.0 * 8000.0;
        if monster_position.distance_squared(hit_position) > MAX_DISTANCE_SQUARED {
            hit_position = monster_position + vec3(0.0, 200.0, 0.0);
        }

        let color = match config.attack_color_source {
            ColorSource::Static => config.static_color,
            ColorSource::Damage => range.map_or(config.static_color, |r| r.settings.color),
            ColorSource::Part => {
                let hzv_info = (((*reg).ebp - 0xac) as *const *const HitzoneInfo).read();
                let part = (*hzv_info).part_index;
                config.part_hzv_color(part)
            }
            ColorSource::Hzv => {
                let hzv_info = (((*reg).ebp - 0xac) as *const *const HitzoneInfo).read();
                let hzv = (*hzv_info).hzv_index;
                config.part_hzv_color(hzv)
            }
        };

        let (scale, duration_mod) =
            range.map_or((1.0, 1.0), |r| (r.settings.scale, r.settings.duration));

        let blast_damage = {
            // Blast should only trigger on the part we're hitting, but this is cheap so check just in case
            let mut damage: i16 = monster.blast_damage().iter().sum();
            if damage > 0 {
                let blast_defense = defense.max(0.3);
                damage = (damage as f32 * blast_defense) as i16;
            }
            damage
        };

        if config.attacks_show {
            let mut attack_damage = (damage_before_defense as f32 * defense) as i16;
            let total_damage = attack_damage + blast_damage;
            if total_damage == 0 && damage_before_defense > 0 {
                attack_damage = 1;
            }
            let attack_settings = DamageSettings::default()
                .with_color(color)
                .with_scale(scale)
                .with_position_offset(state.hit_offset.next())
                .with_duration_modifier(duration_mod);
            let hit = DamageInstance::new(
                attack_damage,
                hit_position,
                &state.num_formatter,
                attack_settings,
            );
            state.hits.push(hit);
        }

        if config.blast_show && blast_damage > 0 {
            let blast_settings = DamageSettings::default()
                .with_color(config.blast.color)
                .with_position_offset(vec2(0.0, -150.0));
            let hit = DamageInstance::new(
                blast_damage,
                hit_position,
                &state.num_formatter,
                blast_settings,
            );
            state.hits.push(hit);
        }
    }
}

fn hook_hit_finalized(addresses: &Addresses) -> Result<NoCbHookPoint> {
    let hook_address = addresses.hit_finalized;
    let builder = NoCbHookBuilder::new(hook_address, HookType::JmpBack(on_hit_finalized));
    let hook_point = unsafe { builder.hook() }?;
    debug!("Hooked at {:#X}", hook_address);
    Ok(hook_point)
}

unsafe extern "cdecl" fn on_poison(reg: *mut Registers, _: usize) {
    unsafe {
        let state = STATE.get_unchecked_mut();
        let config = &state.config;
        let Some(player_info) = state.structs.player_info() else {
            return;
        };
        if !config.poison_show {
            return;
        }
        let monster = state.structs.monster_from_ptr((*reg).esi as *mut u8);
        if monster.area() != player_info.area() {
            return;
        }
        if config.hide_damage_on_small_monsters && !monster.is_large() {
            return;
        }
        let damage = ((*reg).ebx as *const i16).wrapping_byte_add(0x6).read();
        if damage < 1 {
            return;
        }
        let position = monster.pos() + vec3(0.0, 200.0, 0.0);
        let settings = DamageSettings::default().with_color(config.poison.color);
        let tick = DamageInstance::new(damage, position, &state.num_formatter, settings);
        state.poison.push(tick);
    }
}

fn hook_poison(addresses: &Addresses) -> Result<NoCbHookPoint> {
    let hook_address = addresses.poison;
    let builder = NoCbHookBuilder::new(hook_address, HookType::JmpBack(on_poison));
    let hook_point = unsafe { builder.hook() }?;
    debug!("Hooked at {:#X}", hook_address);
    Ok(hook_point)
}

unsafe extern "cdecl" fn on_secret_tech(reg: *mut Registers, _: usize) {
    unsafe {
        let state = STATE.get_unchecked_mut();
        let config = &state.config;
        let Some(player_info) = state.structs.player_info() else {
            return;
        };
        let monster = state.structs.monster_from_ptr((*reg).edi as *mut u8);
        if monster.area() != player_info.area() {
            return;
        }
        if config.hide_damage_on_small_monsters && !monster.is_large() {
            return;
        }
        let old_health = monster.health();
        let new_health = (*reg).eax;
        let damage = old_health - new_health as i16;
        let position = monster.pos() + vec3(0.0, 200.0, 0.0);
        let settings = DamageSettings::default()
            .with_color(config.secret_tech.color)
            .with_scale(config.secret_tech.scale)
            .with_duration_modifier(config.secret_tech.duration);
        let tick = DamageInstance::new(damage, position, &state.num_formatter, settings);
        state.hits.push(tick);
    }
}

fn hook_secret_tech(addresses: &Addresses) -> Result<NoCbHookPoint> {
    let hook_address = addresses.secret_tech;
    let builder = NoCbHookBuilder::new(hook_address, HookType::JmpBack(on_secret_tech));
    let hook_point = unsafe { builder.hook() }?;
    debug!("Hooked at {:#X}", hook_address);
    Ok(hook_point)
}

pub fn init(addresses: &Addresses) -> Result<Vec<NoCbHookPoint>> {
    Ok(vec![
        hook_hit_finalized(addresses)?,
        hook_poison(addresses)?,
        hook_secret_tech(addresses)?,
    ])
}
