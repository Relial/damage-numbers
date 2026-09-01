use std::{path::PathBuf, time::Duration};

use bunny_components::Text;
use bunny_plugin::{
    GameMode, PluginContext,
    bunny_ui::{
        Id, Vec2,
        align::Align2,
        containers::{combo_box::ComboBox, frame::Frame, grid::Grid},
        paint::text::fonts::FontId,
        ui::BunnyUi,
        widgets::{button::Button, drag_value::DragValue, separator::Separator, slider::Slider},
    },
};
use mhfz_structs::MhfzStructs;
use strum::IntoEnumIterator;
use tracing::error;

use crate::{
    address::Addresses,
    animation::{InAnimation, OutAnimation},
    config::{ColorSource, Config, DamageRange},
    damage::{DamageInstance, HitOffset},
    scrolling_text::ScrollingText,
};

// epaint panics if we pass a very large font size
const MAX_FONT_SIZE: f32 = 500.0;

pub struct State {
    pub config: Config,
    pub addresses: Addresses,
    config_path: PathBuf,
    pub structs: MhfzStructs,
    pub damage: Vec<DamageInstance>,
    pub in_animation: InAnimation,
    pub out_animation: OutAnimation,
    pub hit_offset: HitOffset,
    pub ice_age_offset: HitOffset,
    pub misc_offset: HitOffset,
    pub num_formatter: numfmt::Formatter,
    damage_range_update: Option<DamageRangeUpdate>,
    pub scrolling_text: ScrollingText,
    pub recalculate_scrolling_text: bool,
}

impl State {
    pub fn new(context: &PluginContext, addresses: Addresses) -> Self {
        let config_path = context
            .config_dir()
            .join(format!("{}.toml", env!("CARGO_PKG_NAME")));
        let config = Config::load(&config_path).unwrap_or_default();
        let info = context.mhfo_info();
        let structs = MhfzStructs::new(info.address, info.game_mode == GameMode::HighGrade);
        Self {
            config_path,
            addresses,
            config,
            structs,
            damage: Vec::new(),
            in_animation: Default::default(),
            out_animation: Default::default(),
            hit_offset: Default::default(),
            ice_age_offset: Default::default(),
            misc_offset: Default::default(),
            num_formatter: numfmt::Formatter::new()
                .separator(',')
                .unwrap()
                .precision(numfmt::Precision::Decimals(0)),
            damage_range_update: None,
            scrolling_text: ScrollingText::default(),
            recalculate_scrolling_text: false,
        }
    }
}

impl<'a> State {
    pub fn menu(&'a mut self, ui: &mut BunnyUi<'a>) {
        let config = &mut self.config;
        if let Some(update) = self.damage_range_update {
            match update {
                DamageRangeUpdate::Add { index } => {
                    let prev = config
                        .damage_ranges
                        .get(index - 1)
                        .map_or(0.0, |r| r.damage_percent);
                    let next = config
                        .damage_ranges
                        .get(index)
                        .map_or(100.0, |r| r.damage_percent);
                    let new_range = DamageRange {
                        damage_percent: prev.midpoint(next),
                        settings: Default::default(),
                    };
                    config.damage_ranges.insert(index, new_range);
                }
                DamageRangeUpdate::Remove { index } => {
                    if config.damage_ranges.len() > 1 {
                        config.damage_ranges.remove(index);
                    }
                }
            };
            self.damage_range_update = None;
        }

        ui.collapsing("General", |ui| {
            ui.checkbox(&mut config.enable_damage_numbers, "Enable damage numbers");
            ui.checkbox(&mut config.animations, "Animations");
            ui.checkbox(&mut config.own_attacks_only, "Own attacks only");
            ui.checkbox(
                &mut config.hide_damage_on_small_monsters,
                "Hide damage on small monsters",
            );
            ui.checkbox(&mut config.hide_halk_attacks, "Hide Halk attacks");
            ui.horizontal(|ui| {
                ui.label("Duration:");
                ui.add(
                    DragValue::new(&mut config.base_duration_seconds)
                        .range(0.0..=10.0)
                        .speed(0.01)
                        .fixed_decimals(2)
                        .suffix("s"),
                );
            });
            ui.horizontal(|ui| {
                ui.label("Font:");
                ComboBox::from_id(Id::from_salt("font selection"))
                    .selected_text(config.font.to_string())
                    .show_ui(ui, |ui| {
                        // TODO use the actual font for each option when that becomes available in BunnyUi's RichText
                        // Gross!
                        for font in ui.fonts().to_vec() {
                            let name = font.to_string();
                            ui.selectable_value(&mut config.font, font, name);
                        }
                    });
            });
            ui.horizontal(|ui| {
                ui.label("Font size:");
                ui.add(
                    DragValue::new(&mut config.font_size)
                        .range(10.0..=500.0)
                        .speed(0.1)
                        .fixed_decimals(1),
                );
            });
        });

        ui.collapsing("Attacks", |ui| {
            ui.checkbox(&mut config.attacks_show, "Enabled");
            let color_select_resp = ui
                .horizontal(|ui| {
                    ui.label("Color selection:");
                    ComboBox::from_id(Id::from_salt("Color selection"))
                        .selected_text(config.attack_color_source.to_string())
                        .show_ui(ui, |ui| {
                            for source in ColorSource::iter() {
                                ui.selectable_value(
                                    &mut config.attack_color_source,
                                    source,
                                    source.to_string(),
                                );
                            }
                        });
                })
                .response;
            let separator_size = Vec2::new(color_select_resp.rect.width(), 10.0);

            ui.add_sized(separator_size, Separator::default());

            match config.attack_color_source {
                ColorSource::Static => {
                    ui.horizontal(|ui| {
                        ui.label("Static color:");
                        ui.color_edit_button(&mut config.static_color);
                    });
                    ui.add_sized(separator_size, Separator::default());
                }
                ColorSource::Damage => {}
                ColorSource::Part => {
                    ui.label("Parts");
                    Grid::new(Id::from_salt("Part color grid")).show(ui, |ui| {
                        let (lower, higher) = config.part_hzv_colors.split_at_mut(4);
                        for (i, (color_l, color_h)) in lower.iter_mut().zip(higher).enumerate() {
                            ui.horizontal(|ui| {
                                ui.label(format!("{}:", i));
                                ui.color_edit_button(color_l);
                            });
                            ui.horizontal(|ui| {
                                ui.label(format!("{}", i + 4));
                                ui.color_edit_button(color_h);
                            });
                            ui.end_row();
                        }
                    });
                    ui.add_sized(separator_size, Separator::default());
                }
                ColorSource::Hzv => {
                    ui.label("HZVs");
                    Grid::new(Id::from_salt("HZV color grid")).show(ui, |ui| {
                        let (lower, higher) = config.part_hzv_colors.split_at_mut(4);
                        for (i, (color_l, color_h)) in lower.iter_mut().zip(higher).enumerate() {
                            ui.horizontal(|ui| {
                                ui.label(format!("{}:", i));
                                ui.color_edit_button(color_l);
                            });
                            ui.horizontal(|ui| {
                                ui.label(format!("{}", i + 4));
                                ui.color_edit_button(color_h);
                            });
                            ui.end_row();
                        }
                    });
                    ui.add_sized(separator_size, Separator::default());
                }
            }

            ui.label("Styles based on % of max hp the attack dealt");
            ui.label("Highest % used for low health targets (max hp<1,000):");
            ui.add(
                Slider::new(&mut config.max_range_under_thousand_hp, 0.01..=100.0)
                    .fixed_decimals(2)
                    .suffix("%"),
            );

            let ranges = &mut config.damage_ranges;
            let limits: Vec<f32> = ranges.iter().skip(1).map(|r| r.damage_percent).collect();
            let enable_color = config.attack_color_source == ColorSource::Damage;
            let widget_height = ui.spacing().interact_size.y;
            let mut low = 0.0;

            for (i, range) in config.damage_ranges.iter_mut().enumerate() {
                let limit = limits.get(i).copied().unwrap_or(100.0);
                let current_low = low;
                low = range.damage_percent;
                Frame::group(ui.style()).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let prefix = format!("{:.2}-", current_low);
                        ui.add(
                            DragValue::new(&mut range.damage_percent)
                                .range(current_low as f64..=limit as f64)
                                .fixed_decimals(2)
                                .speed(0.005)
                                .prefix(prefix)
                                .suffix("%"),
                        );
                        ui.add_space(10.0);
                        if ui
                            .add_sized([30.0, widget_height], Button::new("-"))
                            .clicked()
                        {
                            self.damage_range_update = Some(DamageRangeUpdate::Remove { index: i });
                        }
                    });

                    if enable_color {
                        range.settings.ui(ui);
                    } else {
                        range.settings.ui_disabled_color(ui);
                    }
                });
                if ui
                    .add_sized([50.0, widget_height], Button::new("+"))
                    .clicked()
                {
                    self.damage_range_update = Some(DamageRangeUpdate::Add { index: i + 1 });
                }
            }
        });

        ui.collapsing("Poison", |ui| {
            ui.checkbox(&mut config.poison_show, "Enabled");
            config.poison.ui(ui);
        });

        ui.collapsing("Ice Age", |ui| {
            ui.checkbox(&mut config.ice_age_show, "Enabled");
            config.ice_age.ui(ui);
        });

        ui.collapsing("Secret Tech", |ui| {
            ui.checkbox(&mut config.secret_tech_show, "Enabled");
            config.secret_tech.ui(ui);
        });

        ui.collapsing("Hexaflash", |ui| {
            ui.checkbox(&mut config.hexaflash_show, "Enabled");
            config.hexaflash.ui(ui);
        });

        ui.collapsing("Misc environmental", |ui| {
            ui.checkbox(&mut config.misc_show, "Enabled");
            config.misc.ui(ui);
        });

        ui.collapsing("Scrolling Damage Text", |ui| {
            self.recalculate_scrolling_text = config.scrolling_text.ui(ui);
        });
    }

    pub fn ui(&mut self, ui: &mut BunnyUi) {
        let camera = ui.camera();
        let config = &self.config;
        let painter = ui.painter();
        let max_rect = ui.max_rect();
        let base_duration = Duration::from_secs_f32(config.base_duration_seconds);

        self.damage.retain(|damage_instance| {
            let duration = base_duration.mul_f32(damage_instance.duration_modifier());
            if duration == Duration::ZERO {
                return false;
            }
            let elapsed = damage_instance.elapsed();
            if let Some((screen_pos, _)) = camera.world_to_screen(damage_instance.position()) {
                let screen_pos = Vec2 {
                    x: screen_pos.x,
                    y: screen_pos.y,
                };
                let remaining = duration.saturating_sub(elapsed);
                let mut text = Text::new(
                    damage_instance.damage(),
                    FontId {
                        family: config.font.clone(),
                        size: (config.font_size * damage_instance.scale()).min(MAX_FONT_SIZE),
                    },
                )
                .with_pos(screen_pos + damage_instance.position_offset())
                .with_pivot(Align2::CENTER_CENTER)
                .with_color(damage_instance.color());
                if let Some(shadow) = damage_instance.shadow() {
                    text = text.with_shadow(shadow);
                }
                if self.config.animations {
                    self.in_animation.apply(elapsed, &mut text);
                    self.out_animation.apply(remaining, &mut text);
                }
                text.paint(painter, max_rect);
            }
            elapsed < duration
        });

        if self.recalculate_scrolling_text {
            self.scrolling_text
                .recalculate_positions(config.scrolling_text.font_size);
        }
        self.scrolling_text.ui(&config.scrolling_text, ui);
    }

    pub fn save_config(&self) {
        if let Err(e) = self.config.save(&self.config_path) {
            error!("Config save error: {e}");
        }
    }
}

#[derive(Clone, Copy)]
enum DamageRangeUpdate {
    Add { index: usize },
    Remove { index: usize },
}
