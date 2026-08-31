use std::path::Path;

use anyhow::Result;
use bunny_components::{EditMenu, TextShadow};
use bunny_plugin::bunny_ui::{
    Color32,
    containers::collapsing_header::CollapsingHeader,
    paint::text::fonts::FontFamily,
    ui::BunnyUi,
    vec2,
    widgets::{drag_value::DragValue, separator::Separator},
};
use serde::{Deserialize, Serialize};
use strum::EnumIter;

const FIRE: Color32 = Color32::from_rgb(255, 72, 2);
const WATER: Color32 = Color32::from_rgb(146, 235, 255);
const ICE: Color32 = Color32::from_rgb(173, 206, 247);
const THUNDER: Color32 = Color32::from_rgb(255, 254, 3);
const DRAGON: Color32 = Color32::from_rgb(107, 114, 182);

#[derive(Clone, Serialize, Deserialize)]
pub struct Config {
    pub base_duration_secs: f32,
    pub animations: bool,
    pub font: FontFamily,
    pub font_size: f32,
    pub attacks_show: bool,
    pub hide_damage_on_small_monsters: bool,
    pub max_range_under_thousand_hp: f32,
    pub own_attacks_only: bool,
    pub hide_halk_attacks: bool,
    pub attack_color_source: ColorSource,
    pub static_color: Color32,
    pub damage_ranges: Vec<DamageRange>,
    pub part_hzv_colors: [Color32; 8],
    pub poison_show: bool,
    pub poison: DamageSettings,
    pub ice_age_show: bool,
    pub ice_age: DamageSettings,
    pub blast_show: bool,
    pub blast: DamageSettings,
    pub secret_tech_show: bool,
    pub secret_tech: DamageSettings,
    pub hexaflash_show: bool,
    pub hexaflash: Hexaflash,
    pub misc_show: bool,
    pub misc: DamageSettings,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            base_duration_secs: 1.5,
            animations: true,
            font: FontFamily::Name("Toronto-Regular".into()),
            font_size: 40.0,
            attacks_show: true,
            hide_damage_on_small_monsters: false,
            max_range_under_thousand_hp: 1.0,
            own_attacks_only: false,
            hide_halk_attacks: false,
            attack_color_source: Default::default(),
            static_color: Color32::LIGHT_YELLOW,
            damage_ranges: vec![
                DamageRange {
                    damage_percent: 0.03,
                    settings: DamageSettings {
                        color: Color32::GRAY,
                        scale: 0.6,
                        duration: 0.6,
                        shadow: Default::default(),
                    },
                },
                DamageRange {
                    damage_percent: 0.3,
                    settings: DamageSettings {
                        color: Color32::LIGHT_YELLOW,
                        scale: 1.0,
                        duration: 1.0,
                        shadow: Default::default(),
                    },
                },
                DamageRange {
                    damage_percent: 1.0,
                    settings: DamageSettings {
                        color: Color32::YELLOW,
                        scale: 1.2,
                        duration: 1.2,
                        shadow: Default::default(),
                    },
                },
                DamageRange {
                    damage_percent: 10.0,
                    settings: DamageSettings {
                        color: Color32::LIGHT_RED,
                        scale: 2.0,
                        duration: 1.5,
                        shadow: Default::default(),
                    },
                },
                DamageRange {
                    damage_percent: 100.0,
                    settings: DamageSettings {
                        color: Color32::RED,
                        scale: 4.0,
                        duration: 2.0,
                        shadow: Default::default(),
                    },
                },
            ],
            part_hzv_colors: [
                Color32::CYAN,
                Color32::ORANGE,
                Color32::LIGHT_GREEN,
                Color32::BLUE,
                Color32::PURPLE,
                Color32::WHITE,
                Color32::LIGHT_RED,
                Color32::from_rgb(226, 160, 255),
            ],
            poison_show: true,
            poison: DamageSettings {
                color: Color32::from_rgb(255, 63, 208),
                scale: 1.0,
                duration: 1.0,
                shadow: Default::default(),
            },
            ice_age_show: true,
            ice_age: DamageSettings {
                color: Color32::LIGHT_BLUE,
                scale: 0.6,
                duration: 0.6,
                shadow: Default::default(),
            },
            blast_show: true,
            blast: DamageSettings {
                color: Color32::ORANGE,
                scale: 1.0,
                duration: 1.0,
                shadow: Default::default(),
            },
            secret_tech_show: true,
            secret_tech: DamageSettings {
                color: Color32::RED,
                scale: 4.0,
                duration: 2.0,
                shadow: Default::default(),
            },
            hexaflash_show: true,
            hexaflash: Default::default(),
            misc_show: true,
            misc: DamageSettings {
                color: Color32::LIGHT_YELLOW,
                scale: 2.0,
                duration: 1.5,
                shadow: Default::default(),
            },
        }
    }
}

impl Config {
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let bytes = std::fs::read(path)?;
        let config = toml::from_slice(&bytes)?;
        Ok(config)
    }

    pub fn save(&self, path: impl AsRef<Path>) -> Result<()> {
        let contents = toml::to_string(self)?;
        std::fs::write(path, contents)?;
        Ok(())
    }

    pub fn get_range(&self, portion_of_max: f32) -> Option<&DamageRange> {
        let percent = portion_of_max * 100.0;
        self.damage_ranges
            .iter()
            .find(|damage_range| percent <= damage_range.damage_percent)
    }

    pub fn part_hzv_color(&self, part: u16) -> Color32 {
        let part = part as usize % self.part_hzv_colors.len();
        self.part_hzv_colors[part]
    }
}

#[derive(Clone, Copy, Default, Debug, PartialEq, Serialize, Deserialize, EnumIter)]
pub enum ColorSource {
    Static,
    #[default]
    Damage,
    Part,
    Hzv,
}

impl std::fmt::Display for ColorSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            ColorSource::Static => "Static",
            ColorSource::Damage => "By damage dealt",
            ColorSource::Part => "By part",
            ColorSource::Hzv => "By hzv",
        };
        write!(f, "{s}")
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct DamageRange {
    pub damage_percent: f32,
    pub settings: DamageSettings,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Hexaflash {
    pub fire: DamageSettings,
    pub water: DamageSettings,
    pub ice: DamageSettings,
    pub thunder: DamageSettings,
    pub dragon: DamageSettings,
    pub raw: DamageSettings,
}

impl Default for Hexaflash {
    fn default() -> Self {
        Self {
            fire: DamageSettings {
                color: FIRE,
                scale: 1.2,
                duration: 1.2,
                shadow: Default::default(),
            },
            water: DamageSettings {
                color: WATER,
                scale: 2.0,
                duration: 1.5,
                shadow: Default::default(),
            },
            ice: DamageSettings {
                color: ICE,
                scale: 2.0,
                duration: 1.5,
                shadow: Default::default(),
            },
            thunder: DamageSettings {
                color: THUNDER,
                scale: 1.0,
                duration: 1.0,
                shadow: Default::default(),
            },
            dragon: DamageSettings {
                color: DRAGON,
                scale: 2.0,
                duration: 1.5,
                shadow: Default::default(),
            },
            raw: DamageSettings {
                color: Color32::WHITE,
                scale: 1.2,
                duration: 1.2,
                shadow: Default::default(),
            },
        }
    }
}

impl Hexaflash {
    pub fn ui<'a>(&'a mut self, ui: &mut BunnyUi<'a>) {
        let separator_size = vec2(100.0, 10.0);
        ui.colored_label(self.fire.color, "Fire");
        self.fire.ui(ui);
        ui.add_sized(separator_size, Separator::default());
        ui.colored_label(self.water.color, "Water");
        self.water.ui(ui);
        ui.add_sized(separator_size, Separator::default());
        ui.colored_label(self.ice.color, "Ice");
        self.ice.ui(ui);
        ui.add_sized(separator_size, Separator::default());
        ui.colored_label(self.thunder.color, "Thunder");
        self.thunder.ui(ui);
        ui.add_sized(separator_size, Separator::default());
        ui.colored_label(self.dragon.color, "Dragon");
        self.dragon.ui(ui);
        ui.add_sized(separator_size, Separator::default());
        ui.colored_label(self.raw.color, "Raw");
        self.raw.ui(ui);
    }

    pub fn settings_from_element_idx(&self, element_idx: u32) -> Option<DamageSettings> {
        match element_idx {
            0 => Some(self.fire),
            1 => Some(self.water),
            2 => Some(self.ice),
            3 => Some(self.thunder),
            4 => Some(self.dragon),
            5 => Some(self.raw),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct DamageSettings {
    pub color: Color32,
    pub scale: f32,
    pub duration: f32,
    pub shadow: Shadow,
}

impl Default for DamageSettings {
    fn default() -> Self {
        Self {
            color: Color32::LIGHT_YELLOW,
            scale: 1.0,
            duration: 1.0,
            shadow: Default::default(),
        }
    }
}

impl DamageSettings {
    pub fn ui<'a>(&'a mut self, ui: &mut BunnyUi<'a>) {
        ui.horizontal(|ui| {
            ui.label("Color:");
            ui.color_edit_button(&mut self.color);
        });
        ui.horizontal(|ui| {
            ui.label("Scale:");
            ui.add(
                DragValue::new(&mut self.scale)
                    .range(0.0..=100.0)
                    .speed(0.01)
                    .fixed_decimals(2)
                    .suffix("x"),
            );
        });
        ui.horizontal(|ui| {
            ui.label("Duration:");
            ui.add(
                DragValue::new(&mut self.duration)
                    .range(0.0..=100.0)
                    .speed(0.01)
                    .fixed_decimals(2)
                    .suffix("x"),
            );
        });
        CollapsingHeader::new("Shadow")
            .id(ui.next_id())
            .show(ui, |ui| {
                ui.checkbox(&mut self.shadow.enabled, "Enabled");
                ui.horizontal(|ui| {
                    ui.label("Color:");
                    ui.color_edit_button(&mut self.shadow.text_shadow.color);
                });
                ui.horizontal(|ui| {
                    ui.label("Offset:");
                    self.shadow.text_shadow.offset.edit_menu(ui);
                });
            });
    }

    pub fn ui_disabled_color<'a>(&'a mut self, ui: &mut BunnyUi<'a>) {
        ui.horizontal(|ui| {
            ui.disable();
            ui.label("Color:");
            ui.color_edit_button(&mut self.color);
        });
        ui.horizontal(|ui| {
            ui.label("Scale:");
            ui.add(
                DragValue::new(&mut self.scale)
                    .range(0.0..=100.0)
                    .speed(0.01)
                    .fixed_decimals(2)
                    .suffix("x"),
            );
        });
        ui.horizontal(|ui| {
            ui.label("Duration:");
            ui.add(
                DragValue::new(&mut self.duration)
                    .range(0.0..=100.0)
                    .speed(0.01)
                    .fixed_decimals(2)
                    .suffix("x"),
            );
        });
        CollapsingHeader::new("Shadow")
            .id(ui.next_id())
            .show(ui, |ui| {
                ui.checkbox(&mut self.shadow.enabled, "Enabled");
                ui.horizontal(|ui| {
                    ui.label("Color:");
                    ui.color_edit_button(&mut self.shadow.text_shadow.color);
                });
                ui.horizontal(|ui| {
                    ui.label("Offset:");
                    self.shadow.text_shadow.offset.edit_menu(ui);
                });
            });
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Shadow {
    pub enabled: bool,
    pub text_shadow: TextShadow,
}

impl Default for Shadow {
    fn default() -> Self {
        Self {
            enabled: true,
            text_shadow: Default::default(),
        }
    }
}
