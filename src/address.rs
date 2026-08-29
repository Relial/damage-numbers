use bunny_plugin::{GameMode, MhfoInfo};

#[derive(Clone, Copy, Debug)]
pub struct Addresses {
    pub hit_finalized: usize,
    pub poison: usize,
    pub secret_tech: usize,
    pub mudslide: usize,
    pub stalactite: usize,
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
                stalactite: dll + 0x8a498a,
            },
            GameMode::HighGrade => todo!(),
        }
    }
}
