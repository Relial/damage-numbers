use bunny_plugin::{GameMode, MhfoInfo};
use mhfz_structs::AttackEffectInfo;

#[derive(Clone, Copy, Debug)]
pub struct Addresses {
    pub hit: usize,
    hit_alt_damage: usize,
    hit_damage_reduction: usize,
    pub poison: usize,
    pub secret_tech: usize,
    pub mudslide: usize,
    pub stalactite: usize,
    pub hexa_general: usize,
    pub hexa_fire_thunder: usize,
    pub hexa_post_fire: usize,
    pub hexa_post_thunder: usize,
    pub hexa_post_raw: usize,
}

impl Addresses {
    pub fn new(mhfo_info: MhfoInfo) -> Self {
        let dll = mhfo_info.address;
        match mhfo_info.game_mode {
            GameMode::LowGrade => Self {
                hit: dll + 0x8a46b7,
                hit_alt_damage: dll + 0x1316d10,
                hit_damage_reduction: dll + 0x17f10,
                poison: dll + 0x8251cb,
                secret_tech: dll + 0x12f4840,
                mudslide: dll + 0x121b0dd,
                stalactite: dll + 0x8a4987,
                hexa_general: dll + 0x13cd8a1,
                hexa_fire_thunder: dll + 0x13cda3e,
                hexa_post_fire: dll + 0x13ce818,
                hexa_post_thunder: dll + 0x13ceb43,
                hexa_post_raw: dll + 0x13cecab,
            },
            GameMode::HighGrade => Self {
                hit: dll + 0x8c0218,
                hit_alt_damage: dll + 0x1336890,
                hit_damage_reduction: dll + 0x320f0,
                poison: dll + 0x83fc4b,
                secret_tech: dll + 0x13141a0,
                mudslide: dll + 0x123a53d,
                stalactite: dll + 0x8c04e8,
                hexa_general: dll + 0x13f2b01,
                hexa_fire_thunder: dll + 0x13f2c9e,
                hexa_post_fire: dll + 0x13f3a78,
                hexa_post_thunder: dll + 0x13f3da3,
                hexa_post_raw: dll + 0x13f3f0b,
            },
        }
    }

    pub fn alt_damage(&self, attack: AttackEffectInfo) -> u32 {
        unsafe {
            let damage: u32;
            core::arch::asm!(
                "call {alt_damage}",
                alt_damage = in(reg) self.hit_alt_damage,
                in("eax") attack.inner(),
                lateout("eax") damage,
                clobber_abi("C"),
            );
            damage
        }
    }

    pub fn damage_reduction(&self, player_addr: u32, mut damage: u32) -> u32 {
        unsafe {
            core::arch::asm!(
                "mov ebx, esi", // Save esi because something something LLVM, the called function doesn't use ebx
                "mov esi, {dmg}",
                "call {damage_reduction}",
                "mov esi, ebx", // Restore esi
                damage_reduction = in(reg) self.hit_damage_reduction,
                dmg = in(reg) damage,
                in("eax") player_addr,
                lateout("eax") damage,
                clobber_abi("C"),
            );
            damage
        }
    }
}
