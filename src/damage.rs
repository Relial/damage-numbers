use std::time::{Duration, Instant};

use bunny_plugin::bunny_ui::{Color32, Vec2, vec2};
use glam::Vec3;

use crate::config::DamageSettings;

pub const OFFSET_SEQUENCE: [Vec2; 8] = [
    vec2(0.0, 0.0),
    vec2(-50.0, 10.0),
    vec2(-10.0, -50.0),
    vec2(40.0, 0.0),
    vec2(30.0, -60.0),
    vec2(-20.0, 15.0),
    vec2(5.0, 40.0),
    vec2(-40.0, -10.0),
];

#[derive(Default)]
pub struct HitOffset(usize);

impl HitOffset {
    pub fn next(&mut self) -> Vec2 {
        let i = self.0;
        self.0 = (self.0 + 1) % OFFSET_SEQUENCE.len();
        OFFSET_SEQUENCE[i]
    }
}

pub struct DamageInstance {
    damage: String,
    position: Vec3,
    time: Instant,
    damage_settings: PaintSettings,
}

impl DamageInstance {
    pub fn new(
        damage: i16,
        position: Vec3,
        formatter: &numfmt::Formatter,
        settings: PaintSettings,
    ) -> Self {
        Self {
            damage: formatter.fmt_string(damage),
            position,
            time: Instant::now(),
            damage_settings: settings,
        }
    }

    #[inline]
    pub fn damage(&self) -> &str {
        &self.damage
    }

    #[inline]
    pub fn position(&self) -> Vec3 {
        self.position
    }

    #[inline]
    pub fn elapsed(&self) -> Duration {
        self.time.elapsed()
    }

    #[inline]
    pub fn position_offset(&self) -> Vec2 {
        self.damage_settings.position_offset
    }

    #[inline]
    pub fn color(&self) -> Color32 {
        self.damage_settings.color
    }

    #[inline]
    pub fn scale(&self) -> f32 {
        self.damage_settings.scale
    }

    #[inline]
    pub fn duration_modifier(&self) -> f32 {
        self.damage_settings.duration_modifier
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PaintSettings {
    pub position_offset: Vec2,
    pub color: Color32,
    pub scale: f32,
    pub duration_modifier: f32,
}

impl Default for PaintSettings {
    fn default() -> Self {
        Self {
            position_offset: Vec2::ZERO,
            color: Color32::LIGHT_YELLOW,
            scale: 1.0,
            duration_modifier: 1.0,
        }
    }
}

impl PaintSettings {
    #[inline]
    pub fn with_position_offset(mut self, position_offset: Vec2) -> Self {
        self.position_offset = position_offset;
        self
    }

    #[inline]
    pub fn with_color(mut self, color: Color32) -> Self {
        self.color = color;
        self
    }

    #[inline]
    pub fn with_scale(mut self, scale: f32) -> Self {
        self.scale = scale;
        self
    }

    #[inline]
    pub fn with_duration_modifier(mut self, duration_modifier: f32) -> Self {
        self.duration_modifier = duration_modifier;
        self
    }

    #[inline]
    pub fn with_damage_settings(mut self, damage_settings: DamageSettings) -> Self {
        let DamageSettings {
            color,
            scale,
            duration,
        } = damage_settings;
        self.color = color;
        self.scale = scale;
        self.duration_modifier = duration;
        self
    }
}
