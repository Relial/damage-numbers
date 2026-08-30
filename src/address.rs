use bunny_plugin::{GameMode, MhfoInfo};

#[derive(Clone, Copy, Debug)]
pub struct Addresses {
    pub hit_finalized: usize,
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
                hit_finalized: dll + 0x8a4c9a,
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
            GameMode::HighGrade => todo!(),
        }
    }
}
