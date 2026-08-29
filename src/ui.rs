use std::{path::PathBuf, time::Duration};

use bunny_components::{Text, TextShadow};
use bunny_plugin::{
    GameMode, PluginContext,
    bunny_ui::{
        Id, Vec2,
        align::Align2,
        containers::{combo_box::ComboBox, frame::Frame, grid::Grid},
        paint::text::fonts::FontId,
        ui::BunnyUi,
        vec2,
        widget_text::RichText,
        widgets::{button::Button, drag_value::DragValue, separator::Separator},
    },
};
use mhfz_structs::MhfzStructs;
use strum::IntoEnumIterator;
use tracing::error;

use crate::{
    animation::{InAnimation, OutAnimation},
    config::{ColorSource, Config, DamageRange},
    damage::{DamageInstance, HitOffset},
};

// epaint panics if we pass a very large font size
const MAX_FONT_SIZE: f32 = 500.0;

pub struct State {
    pub config: Config,
    config_path: PathBuf,
    pub structs: MhfzStructs,
    pub hits: Vec<DamageInstance>,
    pub poison: Vec<DamageInstance>,
    pub ice_age: Vec<DamageInstance>,
    pub in_animation: InAnimation,
    pub out_animation: OutAnimation,
    pub hit_offset: HitOffset,
    pub ice_age_offset: HitOffset,
    pub num_formatter: numfmt::Formatter,
    damage_range_update: Option<DamageRangeUpdate>,
}

impl State {
    pub fn new(context: &PluginContext) -> Self {
        let config_path = context
            .config_dir()
            .join(format!("{}.toml", env!("CARGO_PKG_NAME")));
        // let config = Config::load(&config_path).unwrap_or_default();
        let config = Config::default();
        let info = context.mhfo_info();
        let structs = MhfzStructs::new(info.address, info.game_mode == GameMode::HighGrade);
        Self {
            config_path,
            config,
            structs,
            hits: Vec::new(),
            poison: Vec::new(),
            ice_age: Vec::new(),
            in_animation: Default::default(),
            out_animation: Default::default(),
            hit_offset: Default::default(),
            ice_age_offset: Default::default(),
            num_formatter: numfmt::Formatter::new()
                .separator(',')
                .unwrap()
                .precision(numfmt::Precision::Decimals(0)),
            damage_range_update: None,
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

        Frame::group(ui.style()).show(ui, |ui| {
            ui.label("General");
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
                    DragValue::new(&mut config.base_duration_secs)
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
                        // Gross!
                        // TODO use the actual font for each option when that becomes available in BunnyUi's RichText
                        for font in ui.fonts().to_vec() {
                            let name = font.to_string();
                            ui.selectable_value(&mut config.font, font, RichText::new(name));
                        }
                    });
            });
            ui.horizontal(|ui| {
                ui.label("Font size:");
                ui.add(
                    DragValue::new(&mut config.font_size)
                        .range(10.0..=500.0)
                        .speed(0.01)
                        .fixed_decimals(2),
                );
            });
        });

        Frame::group(ui.style()).show(ui, |ui| {
            ui.label("Attacks");
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

            ui.collapsing("Damage ranges", |ui| {
                ui.label("Style by damage range as percentage of target's max hp");
                ui.label("Targets below 1,000 health are unaffected");
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
                        });

                        if enable_color {
                            range.settings.ui(ui);
                        } else {
                            range.settings.ui_disabled_color(ui);
                        }

                        if ui
                            .add_sized([50.0, widget_height], Button::new("-"))
                            .clicked()
                        {
                            self.damage_range_update = Some(DamageRangeUpdate::Remove { index: i });
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

            match config.attack_color_source {
                ColorSource::Static => {
                    ui.add_sized(separator_size, Separator::default());
                    ui.horizontal(|ui| {
                        ui.label("Static color:");
                        ui.color_edit_button(&mut config.static_color);
                    });
                }
                ColorSource::Damage => {}
                ColorSource::Part => {
                    ui.add_sized(separator_size, Separator::default());
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
                }
                ColorSource::Hzv => {
                    ui.add_sized(separator_size, Separator::default());
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
                }
            }
        });

        Frame::group(ui.style()).show(ui, |ui| {
            ui.label("Poison");
            ui.checkbox(&mut config.poison_show, "Enabled");
            config.poison.ui(ui);
        });

        Frame::group(ui.style()).show(ui, |ui| {
            ui.label("Ice Age");
            ui.checkbox(&mut config.ice_age_show, "Enabled");
            config.ice_age.ui(ui);
        });

        Frame::group(ui.style()).show(ui, |ui| {
            ui.label("Secret Tech");
            ui.checkbox(&mut config.secret_tech_show, "Enabled");
            config.secret_tech.ui(ui);
        });

        Frame::group(ui.style()).show(ui, |ui| {
            ui.label("Hexaflash");
            ui.checkbox(&mut config.hexaflash_show, "Enabled");
            config.hexaflash.ui(ui);
        });

        Frame::group(ui.style()).show(ui, |ui| {
            ui.label("Misc environmental");
            ui.checkbox(&mut config.misc_show, "Enabled");
            config.misc.ui(ui);
        });
    }

    pub fn ui(&mut self, ui: &mut BunnyUi) {
        let camera = ui.camera();
        let config = &self.config;
        let painter = ui.painter();
        let max_rect = ui.max_rect();
        let base_duration = Duration::from_secs_f32(config.base_duration_secs);

        self.hits.retain(|hit| {
            let duration = base_duration.mul_f32(hit.duration_modifier());
            if duration == Duration::ZERO {
                return false;
            }
            let elapsed = hit.elapsed();
            if let Some((screen_pos, _)) = camera.world_to_screen(hit.position()) {
                let screen_pos = Vec2 {
                    x: screen_pos.x,
                    y: screen_pos.y,
                };
                let remaining = duration.saturating_sub(elapsed);
                let mut text = Text::new(
                    hit.damage(),
                    FontId {
                        family: config.font.clone(),
                        size: (config.font_size * hit.scale()).min(MAX_FONT_SIZE),
                    },
                )
                .with_shadow(TextShadow::default())
                .with_pos(screen_pos + hit.position_offset())
                .with_pivot(Align2::CENTER_CENTER)
                .with_color(hit.color());
                if self.config.animations {
                    self.in_animation.apply(elapsed, &mut text);
                    self.out_animation.apply(remaining, &mut text);
                }
                text.paint(painter, max_rect);
            }
            elapsed < duration
        });

        let poison_scale = (config.font_size * config.poison.scale).min(MAX_FONT_SIZE);
        let poison_duration = base_duration.mul_f32(config.poison.duration);
        if poison_duration == Duration::ZERO {
            self.poison.clear();
        }
        self.poison.retain(|poison| {
            let elapsed = poison.elapsed();
            if let Some((screen_pos, _)) = camera.world_to_screen(poison.position()) {
                let screen_pos = Vec2 {
                    x: screen_pos.x,
                    y: screen_pos.y,
                };
                let remaining = poison_duration.saturating_sub(elapsed);
                let mut text = Text::new(
                    poison.damage(),
                    FontId {
                        family: config.font.clone(),
                        size: poison_scale,
                    },
                )
                .with_shadow(TextShadow::default())
                .with_pos(screen_pos - vec2(100.0, 0.0))
                .with_pivot(Align2::CENTER_CENTER)
                .with_color(poison.color());
                if self.config.animations {
                    self.in_animation.apply(elapsed, &mut text);
                    self.out_animation.apply(remaining, &mut text);
                }
                text.paint(painter, max_rect);
            }
            elapsed < poison_duration
        });

        let ice_age_scale = (config.font_size * config.ice_age.scale).min(MAX_FONT_SIZE);
        let ice_age_duration = base_duration.mul_f32(config.ice_age.duration);
        if ice_age_duration == Duration::ZERO {
            self.ice_age.clear();
        }
        self.ice_age.retain(|ice_age| {
            let elapsed = ice_age.elapsed();
            if let Some((screen_pos, _)) = camera.world_to_screen(ice_age.position()) {
                let screen_pos = Vec2 {
                    x: screen_pos.x,
                    y: screen_pos.y,
                };
                let remaining = ice_age_duration.saturating_sub(elapsed);
                let mut text = Text::new(
                    ice_age.damage(),
                    FontId {
                        family: config.font.clone(),
                        size: ice_age_scale,
                    },
                )
                .with_shadow(TextShadow::default())
                .with_pos(screen_pos + vec2(100.0, 0.0) + ice_age.position_offset())
                .with_pivot(Align2::CENTER_CENTER)
                .with_color(ice_age.color());
                if self.config.animations {
                    self.in_animation.apply(elapsed, &mut text);
                    self.out_animation.apply(remaining, &mut text);
                }
                text.paint(painter, max_rect);
            }
            elapsed < ice_age_duration
        });
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
