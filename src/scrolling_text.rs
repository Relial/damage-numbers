use bunny_components::{EditMenu, RelativePosition, Text};
use bunny_plugin::bunny_ui::{
    Color32, Id, Vec2,
    align::Align2,
    containers::{collapsing_header::CollapsingHeader, combo_box::ComboBox},
    paint::{
        stroke::{Stroke, StrokeKind},
        text::fonts::{FontFamily, FontId},
    },
    ui::BunnyUi,
    vec2,
    widgets::drag_value::DragValue,
};
use serde::{Deserialize, Serialize};

use crate::config::Shadow;

#[derive(Debug, Default)]
pub struct ScrollingText {
    entries: Vec<ScrollingTextEntry>,
}

impl ScrollingText {
    #[inline]
    pub fn add(&mut self, entry: ScrollingTextEntry, font_size: f32) {
        // Check if there's space to draw the new entry
        let adjust = self
            .entries
            .last()
            .map_or(0.0, |last| font_size - last.scrolled_distance);
        // If not, adjust all entries further into their scroll
        if adjust > 0.0 {
            for entry in &mut self.entries {
                entry.scrolled_distance += adjust;
            }
        }
        self.entries.push(entry);
    }

    pub fn ui(&mut self, config: &ScrollingTextConfig, ui: &mut BunnyUi) {
        if !config.enabled {
            self.entries.clear();
            return;
        }
        let area = &config.area;
        let max_rect = ui.max_rect();
        let area_pos = area.position.pos_in_rect(&max_rect);
        let rect = config.area.pivot.anchor_size(area_pos, area.size);
        let painter = ui.painter_at(rect);

        painter.rect(
            rect,
            area.corner_radius,
            area.background_color,
            area.border,
            StrokeKind::Inside,
        );

        let margin = config.area.margin as f32;
        let (start_pos, end_pos) = match config.direction {
            ScrollDirection::Up => (
                rect.center_bottom() - vec2(0.0, margin),
                rect.center_top() + vec2(0.0, margin) + vec2(0.0, config.font_size + 5.0), // Font sizes are fake,
            ),
            ScrollDirection::Down => (
                rect.center_top() + vec2(0.0, margin),
                rect.center_bottom() - vec2(0.0, margin) - vec2(0.0, config.font_size + 5.0),
            ),
        };
        let total_scroll_distance = (end_pos.y - start_pos.y).abs();
        let shadow = config
            .text_shadow
            .enabled
            .then_some(config.text_shadow.text_shadow);
        let pivot = match config.direction {
            ScrollDirection::Up => Align2::CENTER_BOTTOM,
            ScrollDirection::Down => Align2::CENTER_TOP,
        };
        self.entries.retain_mut(|entry| {
            if entry.scrolled_distance > total_scroll_distance {
                return false;
            }
            let y = match config.direction {
                ScrollDirection::Up => start_pos.y - entry.scrolled_distance,
                ScrollDirection::Down => start_pos.y + entry.scrolled_distance,
            } + config.text_offset;
            let pos = vec2(start_pos.x, y);

            let mut text = Text::new(
                entry.damage.as_str(),
                FontId {
                    family: config.font.clone(),
                    size: config.font_size,
                },
            )
            .with_pos(pos)
            .with_pivot(pivot)
            .with_color(entry.color);
            if let Some(shadow) = shadow {
                text = text.with_shadow(shadow);
            }

            config.apply_fade(&mut text, entry.scrolled_distance, total_scroll_distance);

            text.paint(&painter, max_rect);

            entry.scrolled_distance += config.speed;
            true
        });
    }
}

#[derive(Debug)]
pub struct ScrollingTextEntry {
    damage: String,
    color: Color32,
    scrolled_distance: f32,
}

impl ScrollingTextEntry {
    pub fn new(damage: i16, color: Color32, formatter: &numfmt::Formatter) -> Self {
        Self {
            damage: formatter.fmt_string(damage),
            color,
            scrolled_distance: 0.0,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ScrollingTextConfig {
    pub enabled: bool,
    pub area: AreaSettings,
    pub speed: f32,
    pub direction: ScrollDirection,
    pub font: FontFamily,
    pub font_size: f32,
    pub text_offset: f32,
    pub text_shadow: Shadow,
    pub fade_in: bool,
    pub fade_in_height: f32,
    pub fade_out: bool,
    pub fade_out_height: f32,
}

impl Default for ScrollingTextConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            area: Default::default(),
            speed: 5.0,
            direction: Default::default(),
            font: FontFamily::Name("Toronto-Regular".into()),
            font_size: 30.0,
            text_offset: 5.0,
            text_shadow: Default::default(),
            fade_in: false,
            fade_in_height: 50.0,
            fade_out: true,
            fade_out_height: 50.0,
        }
    }
}

impl ScrollingTextConfig {
    pub fn ui<'a>(&'a mut self, ui: &mut BunnyUi<'a>) {
        ui.checkbox(&mut self.enabled, "Enabled");
        CollapsingHeader::new("Area")
            .id(ui.next_id())
            .show(ui, |ui| {
                self.area.ui(ui);
            });
        ui.horizontal(|ui| {
            ui.label("Speed:");
            ui.add(
                DragValue::new(&mut self.speed)
                    .fixed_decimals(1)
                    .range(0.0..=1000.0)
                    .speed(0.01),
            );
        });
        ui.horizontal(|ui| {
            ui.label("Scroll direction:");
            ui.radio_value(&mut self.direction, ScrollDirection::Up, "Up");
            ui.radio_value(&mut self.direction, ScrollDirection::Down, "Down");
        });
        ui.horizontal(|ui| {
            ui.label("Font:");
            ComboBox::from_id(Id::from_salt("font selection"))
                .selected_text(self.font.to_string())
                .show_ui(ui, |ui| {
                    // TODO use the actual font for each option when that becomes available in BunnyUi's RichText
                    // Gross!
                    for font in ui.fonts().to_vec() {
                        let name = font.to_string();
                        ui.selectable_value(&mut self.font, font, name);
                    }
                });
        });
        ui.horizontal(|ui| {
            ui.label("Font size:");
            ui.add(
                DragValue::new(&mut self.font_size)
                    .range(10.0..=500.0)
                    .speed(0.1)
                    .fixed_decimals(1),
            );
        });
        ui.horizontal(|ui| {
            ui.label("Text Y offset:");
            ui.add(
                DragValue::new(&mut self.text_offset)
                    .fixed_decimals(1)
                    .speed(0.01)
                    .range(-100.0..=100.0),
            );
        });
        CollapsingHeader::new("Text shadow")
            .id(ui.next_id())
            .show(ui, |ui| {
                ui.checkbox(&mut self.text_shadow.enabled, "Enabled");
                ui.horizontal(|ui| {
                    ui.label("Color:");
                    ui.color_edit_button(&mut self.text_shadow.text_shadow.color);
                });
                ui.horizontal(|ui| {
                    ui.label("Offset:");
                    self.text_shadow.text_shadow.offset.edit_menu(ui);
                });
            });
        ui.checkbox(&mut self.fade_in, "Fade in");
        ui.horizontal(|ui| {
            ui.label("Fade in height:");
            ui.add(DragValue::new(&mut self.fade_in_height).fixed_decimals(0));
        });
        ui.checkbox(&mut self.fade_out, "Fade out");
        ui.horizontal(|ui| {
            ui.label("Fade out height:");
            ui.add(DragValue::new(&mut self.fade_out_height).fixed_decimals(0));
        });
    }

    fn apply_fade(&self, text: &mut Text, scrolled: f32, total_scroll_distance: f32) {
        if self.fade_in && scrolled < self.fade_in_height {
            let t = scrolled / self.fade_in_height;
            text.text_color = Color32::TRANSPARENT.lerp_to_gamma(text.text_color, t);
            if let Some(highlight_color) = text.highlight_color_mut() {
                *highlight_color = Color32::TRANSPARENT.lerp_to_gamma(*highlight_color, t);
            }
        } else if self.fade_out {
            let remaining = total_scroll_distance - scrolled;
            if remaining < self.fade_out_height {
                let t = 1.0 - (remaining / self.fade_out_height);
                text.text_color = text.text_color.lerp_to_gamma(Color32::TRANSPARENT, t);
                if let Some(highlight_color) = text.highlight_color_mut() {
                    *highlight_color = highlight_color.lerp_to_gamma(Color32::TRANSPARENT, t);
                }
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum ScrollDirection {
    #[default]
    Up,
    Down,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct AreaSettings {
    pub position: RelativePosition,
    pub pivot: Align2,
    pub size: Vec2,
    pub background_color: Color32,
    pub border: Stroke,
    pub corner_radius: u8,
    pub margin: u8,
}

impl Default for AreaSettings {
    fn default() -> Self {
        Self {
            position: RelativePosition::new(Align2::CENTER_TOP, vec2(0.0, 50.0)),
            pivot: Align2::CENTER_TOP,
            size: Vec2 { x: 250.0, y: 400.0 },
            background_color: Color32::TRANSPARENT,
            border: Stroke {
                width: 1.0,
                color: Color32::TRANSPARENT,
            },
            corner_radius: 20,
            margin: 3,
        }
    }
}

impl AreaSettings {
    pub fn ui<'a>(&'a mut self, ui: &mut BunnyUi<'a>) {
        self.position.edit_menu(ui);
        ui.horizontal(|ui| {
            ui.label("Pivot:");
            self.pivot.edit_menu(ui);
        });
        ui.horizontal(|ui| {
            ui.label("Area size:");
            self.size.edit_menu(ui);
        });
        ui.label("Border");
        ui.indent(|ui| {
            ui.horizontal(|ui| {
                ui.label("Width:");
                ui.add(DragValue::new(&mut self.border.width).fixed_decimals(0));
            });
            ui.horizontal(|ui| {
                ui.label("Color:");
                ui.color_edit_button(&mut self.border.color);
            });
        });
        ui.horizontal(|ui| {
            ui.label("Background color:");
            ui.color_edit_button(&mut self.background_color);
        });
        ui.horizontal(|ui| {
            ui.label("Corner radius");
            ui.add(DragValue::new(&mut self.corner_radius));
        });
        ui.horizontal(|ui| {
            ui.label("Margin:");
            ui.add(
                DragValue::new(&mut self.margin)
                    .speed(0.01)
                    .fixed_decimals(0),
            );
        });
    }
}
