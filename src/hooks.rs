use anyhow::Result;
use bunny_plugin::{
    bunny_ui::vec2,
    hook_builder::{NoCbHookBuilder, NoCbHookPoint},
};
use glam::vec3;
use ilhook::x86::{HookType, Registers};
use mhfz_structs::{AttackEffectInfo, Entity, HitzoneInfo, Monster};
use tracing::debug;

use crate::{
    address::Addresses,
    config::{ColorSource, Shadow},
    damage::{DamageInstance, PaintSettings},
    plugin::STATE,
    scrolling_text::ScrollingTextEntry,
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
        let settings = PaintSettings::from_damage_settings(config.ice_age)
            .with_position_offset(vec2(100.0, 0.0) + state.ice_age_offset.next());
        if config.enable_damage_numbers {
            let tick = DamageInstance::new(damage, position, &state.num_formatter, settings);
            state.damage.push(tick);
        }

        if config.scrolling_text.enabled {
            let entry = ScrollingTextEntry::new(damage, settings.color, &state.num_formatter);
            state
                .scrolling_text
                .add(entry, config.scrolling_text.font_size);
        }
    }
}

fn hit_checks(reg: *mut Registers, attack: AttackEffectInfo, monster: Monster) -> bool {
    unsafe {
        let a = (((*reg).ebp - 0x96) as *const u8).read();
        if a != 0 {
            return false;
        }

        let att = attack.inner();
        let a = att.wrapping_byte_add(0x9a).read();
        if a & 4 != 0 {
            return false;
        }

        let a = att.wrapping_byte_add(0xf5).read();
        let b = monster.inner().wrapping_byte_add(0x4e4).read();
        if a != 0 && b != 0 && (a ^ b) & 1 == 0 {
            return false;
        }

        let a4 = (att.wrapping_byte_add(0x4) as *const u16).read();
        let b6 = (att.wrapping_byte_add(0x6) as *const u16).read();
        !(a4 == 3 && b6 == 0x21)
    }
}

fn hit_damage_check(attack: AttackEffectInfo, addresses: &Addresses) -> Option<u32> {
    unsafe {
        let att = attack.inner();
        let a = (att.wrapping_add(0x4) as *const u16).read();
        let b = (att.wrapping_add(0x6) as *const u16).read();
        let c = (att as *const u16).read();
        let d = attack.attacker_addr();
        if a == 0x65 && b == 0xa && c != 0 && d != 0 {
            Some(addresses.alt_damage(attack))
        } else {
            None
        }
    }
}

unsafe extern "cdecl" fn on_hit(reg: *mut Registers, _: usize) {
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

        let attack_ptr = (((*reg).ebp - 0x88) as *const *mut u8).read();
        let attack = AttackEffectInfo::new(attack_ptr);
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
        if !hit_checks(reg, attack, monster) {
            return;
        }

        let mut damage_before_defense = hit_damage_check(attack, &state.addresses)
            .unwrap_or((((*reg).ebp - 0x94) as *const u32).read());
        let player_addr = (((*reg).ebp - 0x84) as *const u32).read();
        damage_before_defense = state
            .addresses
            .damage_reduction(player_addr, damage_before_defense);
        damage_before_defense = damage_before_defense.min(0x7fff);

        let defense = monster.defense_multiplier();
        let hit_damage = (damage_before_defense as f32 * defense) as i16;
        let max_health = monster.max_health();
        let mut portion_of_max = hit_damage as f32 / max_health as f32;
        if max_health < 1000 {
            portion_of_max = portion_of_max.min(config.max_range_under_thousand_hp / 100.0);
        }
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

        let (scale, duration_mod, shadow) = range.map_or_else(
            || (1.0, 1.0, Shadow::default()),
            |r| (r.settings.scale, r.settings.duration, r.settings.shadow),
        );

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
            let attack_settings = PaintSettings::default()
                .with_color(color)
                .with_scale(scale)
                .with_position_offset(state.hit_offset.next())
                .with_duration_modifier(duration_mod)
                .with_shadow(shadow);
            if config.enable_damage_numbers {
                let hit = DamageInstance::new(
                    attack_damage,
                    hit_position,
                    &state.num_formatter,
                    attack_settings,
                );
                state.damage.push(hit);
            }

            if config.scrolling_text.enabled {
                let entry = ScrollingTextEntry::new(
                    attack_damage,
                    attack_settings.color,
                    &state.num_formatter,
                );
                state
                    .scrolling_text
                    .add(entry, config.scrolling_text.font_size);
            }
        }

        if config.blast_show && blast_damage > 0 {
            let blast_settings = PaintSettings::from_damage_settings(config.blast)
                .with_position_offset(vec2(0.0, -150.0));
            if config.enable_damage_numbers {
                let hit = DamageInstance::new(
                    blast_damage,
                    hit_position,
                    &state.num_formatter,
                    blast_settings,
                );
                state.damage.push(hit);
            }

            if config.scrolling_text.enabled {
                let entry = ScrollingTextEntry::new(
                    blast_damage,
                    blast_settings.color,
                    &state.num_formatter,
                );
                state
                    .scrolling_text
                    .add(entry, config.scrolling_text.font_size);
            }
        }
    }
}

fn hook_hit(addresses: &Addresses) -> Result<NoCbHookPoint> {
    let hook_address = addresses.hit;
    let builder = NoCbHookBuilder::new(hook_address, HookType::JmpBack(on_hit));
    let hook_point = unsafe { builder.hook() }?;
    debug!("Hooked at {:#X}", hook_address);
    Ok(hook_point)
}

unsafe extern "cdecl" fn on_poison(reg: *mut Registers, _: usize) {
    unsafe {
        let state = STATE.get_unchecked_mut();
        let config = &state.config;
        if !config.poison_show {
            return;
        }
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
        let damage = ((*reg).ebx as *const i16).wrapping_byte_add(0x6).read();
        if damage < 1 {
            return;
        }
        let position = monster.pos() + vec3(0.0, 200.0, 0.0);
        let settings = PaintSettings::from_damage_settings(config.poison)
            .with_position_offset(vec2(-100.0, 0.0));
        if config.enable_damage_numbers {
            let damage_instance =
                DamageInstance::new(damage, position, &state.num_formatter, settings);
            state.damage.push(damage_instance);
        }

        if config.scrolling_text.enabled {
            let entry = ScrollingTextEntry::new(damage, settings.color, &state.num_formatter);
            state
                .scrolling_text
                .add(entry, config.scrolling_text.font_size);
        }
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
        if !config.secret_tech_show {
            return;
        }
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
        if damage < 1 {
            return;
        }
        let position = monster.pos() + vec3(0.0, 200.0, 0.0);
        let settings = PaintSettings::from_damage_settings(config.secret_tech);
        if config.enable_damage_numbers {
            let damage_instance =
                DamageInstance::new(damage, position, &state.num_formatter, settings);
            state.damage.push(damage_instance);
        }

        if config.scrolling_text.enabled {
            let entry = ScrollingTextEntry::new(damage, settings.color, &state.num_formatter);
            state
                .scrolling_text
                .add(entry, config.scrolling_text.font_size);
        }
    }
}

fn hook_secret_tech(addresses: &Addresses) -> Result<NoCbHookPoint> {
    let hook_address = addresses.secret_tech;
    let builder = NoCbHookBuilder::new(hook_address, HookType::JmpBack(on_secret_tech));
    let hook_point = unsafe { builder.hook() }?;
    debug!("Hooked at {:#X}", hook_address);
    Ok(hook_point)
}

unsafe extern "cdecl" fn on_mudslide(reg: *mut Registers, _: usize) {
    unsafe {
        let state = STATE.get_unchecked_mut();
        let config = &state.config;
        if !config.misc_show {
            return;
        }
        let Some(player_info) = state.structs.player_info() else {
            return;
        };
        let monster = state.structs.monster_from_ptr((*reg).ecx as *mut u8);
        if monster.area() != player_info.area() {
            return;
        }
        if config.hide_damage_on_small_monsters && !monster.is_large() {
            return;
        }
        let old_health = monster.health();
        let new_health = (*reg).eax;
        let damage = old_health - new_health as i16;
        if damage < 1 {
            return;
        }
        let position = monster.pos() + vec3(0.0, 200.0, 0.0);
        let settings = PaintSettings::from_damage_settings(config.misc)
            .with_position_offset(state.misc_offset.next() * 2.0);
        if config.enable_damage_numbers {
            let damage_instance =
                DamageInstance::new(damage, position, &state.num_formatter, settings);
            state.damage.push(damage_instance);
        }

        if config.scrolling_text.enabled {
            let entry = ScrollingTextEntry::new(damage, settings.color, &state.num_formatter);
            state
                .scrolling_text
                .add(entry, config.scrolling_text.font_size);
        }
    }
}

fn hook_mudslide(addresses: &Addresses) -> Result<NoCbHookPoint> {
    let hook_address = addresses.mudslide;
    let builder = NoCbHookBuilder::new(hook_address, HookType::JmpBack(on_mudslide));
    let hook_point = unsafe { builder.hook() }?;
    debug!("Hooked at {:#X}", hook_address);
    Ok(hook_point)
}

unsafe extern "cdecl" fn on_stalactite(reg: *mut Registers, _: usize) {
    unsafe {
        let state = STATE.get_unchecked_mut();
        let config = &state.config;
        if !config.misc_show {
            return;
        }
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
        let damage = (*reg).edx as i16;
        if damage < 1 {
            return;
        }
        let position = monster.pos() + vec3(0.0, 200.0, 0.0);
        let settings = PaintSettings::from_damage_settings(config.misc)
            .with_position_offset(state.misc_offset.next() * 2.0);
        if config.enable_damage_numbers {
            let damage_instance =
                DamageInstance::new(damage, position, &state.num_formatter, settings);
            state.damage.push(damage_instance);
        }

        if config.scrolling_text.enabled {
            let entry = ScrollingTextEntry::new(damage, settings.color, &state.num_formatter);
            state
                .scrolling_text
                .add(entry, config.scrolling_text.font_size);
        }
    }
}

fn hook_stalactite(addresses: &Addresses) -> Result<NoCbHookPoint> {
    let hook_address = addresses.stalactite;
    let builder = NoCbHookBuilder::new(hook_address, HookType::JmpBack(on_stalactite));
    let hook_point = unsafe { builder.hook() }?;
    debug!("Hooked at {:#X}", hook_address);
    Ok(hook_point)
}

unsafe extern "cdecl" fn on_hexa_general(reg: *mut Registers, _: usize) {
    unsafe {
        let state = STATE.get_unchecked_mut();
        let config = &state.config;
        if !config.hexaflash_show {
            return;
        }
        let element_idx = (*reg).edi;
        // Fire and Thunder have their own handling
        if element_idx == 0 || element_idx == 3 {
            return;
        }
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
        let damage = (*reg).eax as i16;
        if damage < 1 {
            return;
        }
        let Some(settings) = config.hexaflash.settings_from_element_idx(element_idx) else {
            return;
        };
        let position = monster.pos() + vec3(0.0, 200.0, 0.0);
        let settings = PaintSettings::from_damage_settings(settings);
        if config.enable_damage_numbers {
            let damage_instance =
                DamageInstance::new(damage, position, &state.num_formatter, settings);
            state.damage.push(damage_instance);
        }

        if config.scrolling_text.enabled {
            let entry = ScrollingTextEntry::new(damage, settings.color, &state.num_formatter);
            state
                .scrolling_text
                .add(entry, config.scrolling_text.font_size);
        }
    }
}

fn hook_hexa_general(addresses: &Addresses) -> Result<NoCbHookPoint> {
    let hook_address = addresses.hexa_general;
    let builder = NoCbHookBuilder::new(hook_address, HookType::JmpBack(on_hexa_general));
    let hook_point = unsafe { builder.hook() }?;
    debug!("Hooked at {:#X}", hook_address);
    Ok(hook_point)
}

unsafe extern "cdecl" fn on_hexa_fire_thunder(reg: *mut Registers, _: usize) {
    unsafe {
        let state = STATE.get_unchecked_mut();
        let config = &state.config;
        if !config.hexaflash_show {
            return;
        }
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
        let damage = (*reg).eax as i16;
        if damage < 1 {
            return;
        }
        let element_check = (((*reg).ebp + 0xc) as *const u32).read();
        let settings = if element_check == 3 {
            config.hexaflash.thunder
        } else if element_check == 0x1a {
            config.hexaflash.fire
        } else {
            return;
        };
        let position = monster.pos() + vec3(0.0, 200.0, 0.0);
        let settings = PaintSettings::from_damage_settings(settings);
        if config.enable_damage_numbers {
            let damage_instance =
                DamageInstance::new(damage, position, &state.num_formatter, settings);
            state.damage.push(damage_instance);
        }

        if config.scrolling_text.enabled {
            let entry = ScrollingTextEntry::new(damage, settings.color, &state.num_formatter);
            state
                .scrolling_text
                .add(entry, config.scrolling_text.font_size);
        }
    }
}

fn hook_hexa_fire_thunder(addresses: &Addresses) -> Result<NoCbHookPoint> {
    let hook_address = addresses.hexa_fire_thunder;
    let builder = NoCbHookBuilder::new(hook_address, HookType::JmpBack(on_hexa_fire_thunder));
    let hook_point = unsafe { builder.hook() }?;
    debug!("Hooked at {:#X}", hook_address);
    Ok(hook_point)
}

unsafe extern "cdecl" fn on_hexa_post_fire(reg: *mut Registers, _: usize) {
    unsafe {
        let state = STATE.get_unchecked_mut();
        let config = &state.config;
        if !config.hexaflash_show {
            return;
        }
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
        let damage = (*reg).edx as i16;
        if damage < 1 {
            return;
        }
        let settings = config.hexaflash.fire;
        let position = monster.pos() + vec3(0.0, 200.0, 0.0);
        let settings = PaintSettings::from_damage_settings(settings);
        if config.enable_damage_numbers {
            let damage_instance =
                DamageInstance::new(damage, position, &state.num_formatter, settings);
            state.damage.push(damage_instance);
        }

        if config.scrolling_text.enabled {
            let entry = ScrollingTextEntry::new(damage, settings.color, &state.num_formatter);
            state
                .scrolling_text
                .add(entry, config.scrolling_text.font_size);
        }
    }
}

fn hook_hexa_post_fire(addresses: &Addresses) -> Result<NoCbHookPoint> {
    let hook_address = addresses.hexa_post_fire;
    let builder = NoCbHookBuilder::new(hook_address, HookType::JmpBack(on_hexa_post_fire));
    let hook_point = unsafe { builder.hook() }?;
    debug!("Hooked at {:#X}", hook_address);
    Ok(hook_point)
}

unsafe extern "cdecl" fn on_hexa_post_thunder(reg: *mut Registers, _: usize) {
    unsafe {
        let state = STATE.get_unchecked_mut();
        let config = &state.config;
        if !config.hexaflash_show {
            return;
        }
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
        let damage = (*reg).eax as i16;
        if damage < 1 {
            return;
        }
        let settings = config.hexaflash.thunder;
        let position = monster.pos() + vec3(0.0, 200.0, 0.0);
        let settings = PaintSettings::from_damage_settings(settings);
        if config.enable_damage_numbers {
            let damage_instance =
                DamageInstance::new(damage, position, &state.num_formatter, settings);
            state.damage.push(damage_instance);
        }

        if config.scrolling_text.enabled {
            let entry = ScrollingTextEntry::new(damage, settings.color, &state.num_formatter);
            state
                .scrolling_text
                .add(entry, config.scrolling_text.font_size);
        }
    }
}

fn hook_hexa_post_thunder(addresses: &Addresses) -> Result<NoCbHookPoint> {
    let hook_address = addresses.hexa_post_thunder;
    let builder = NoCbHookBuilder::new(hook_address, HookType::JmpBack(on_hexa_post_thunder));
    let hook_point = unsafe { builder.hook() }?;
    debug!("Hooked at {:#X}", hook_address);
    Ok(hook_point)
}

unsafe extern "cdecl" fn on_hexa_post_raw(reg: *mut Registers, _: usize) {
    unsafe {
        let state = STATE.get_unchecked_mut();
        let config = &state.config;
        if !config.hexaflash_show {
            return;
        }
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
        let damage = (*reg).edi as i16;
        if damage < 1 {
            return;
        }
        let settings = config.hexaflash.raw;
        let position = monster.pos() + vec3(0.0, 200.0, 0.0);
        let settings = PaintSettings::from_damage_settings(settings);
        if config.enable_damage_numbers {
            let damage_instance =
                DamageInstance::new(damage, position, &state.num_formatter, settings);
            state.damage.push(damage_instance);
        }

        if config.scrolling_text.enabled {
            let entry = ScrollingTextEntry::new(damage, settings.color, &state.num_formatter);
            state
                .scrolling_text
                .add(entry, config.scrolling_text.font_size);
        }
    }
}

fn hook_hexa_post_raw(addresses: &Addresses) -> Result<NoCbHookPoint> {
    let hook_address = addresses.hexa_post_raw;
    let builder = NoCbHookBuilder::new(hook_address, HookType::JmpBack(on_hexa_post_raw));
    let hook_point = unsafe { builder.hook() }?;
    debug!("Hooked at {:#X}", hook_address);
    Ok(hook_point)
}

pub fn init(addresses: &Addresses) -> Result<Vec<NoCbHookPoint>> {
    Ok(vec![
        hook_hit(addresses)?,
        hook_poison(addresses)?,
        hook_secret_tech(addresses)?,
        hook_mudslide(addresses)?,
        hook_stalactite(addresses)?,
        hook_hexa_general(addresses)?,
        hook_hexa_fire_thunder(addresses)?,
        hook_hexa_post_fire(addresses)?,
        hook_hexa_post_thunder(addresses)?,
        hook_hexa_post_raw(addresses)?,
    ])
}
