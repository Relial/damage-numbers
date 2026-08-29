use std::time::Duration;

use bunny_components::Text;
use bunny_plugin::bunny_ui::Color32;

struct SmoothStepIn;
struct SmoothStepOut;

impl SmoothStepIn {
    #[inline]
    fn sample(t: f32) -> f32 {
        ((1.5 - 0.5 * t) * t) * t
    }
}

impl SmoothStepOut {
    #[inline]
    fn sample(t: f32) -> f32 {
        (1.5 + (-0.5 * t) * t) * t
    }
}

pub struct InAnimation {
    pub duration: Duration,
    pub max_offset: f32,
}

impl Default for InAnimation {
    fn default() -> Self {
        Self {
            duration: Duration::from_millis(300),
            max_offset: 30.0,
        }
    }
}

impl InAnimation {
    pub fn apply(&self, elapsed: Duration, text: &mut Text) {
        if elapsed < self.duration {
            let t = elapsed.div_duration_f32(self.duration);
            let t = SmoothStepOut::sample(t);
            text.position.offset.y += (1.0 - t) * self.max_offset;
            text.text_color = Color32::TRANSPARENT.lerp_to_gamma(text.text_color, t);
            if let Some(highlight_color) = text.highlight_color_mut() {
                *highlight_color = Color32::TRANSPARENT.lerp_to_gamma(*highlight_color, t);
            }
        }
    }
}

pub struct OutAnimation {
    pub duration: Duration,
    pub max_offset: f32,
}

impl Default for OutAnimation {
    fn default() -> Self {
        Self {
            duration: Duration::from_millis(150),
            max_offset: 30.0,
        }
    }
}

impl OutAnimation {
    pub fn apply(&self, remaining: Duration, text: &mut Text) {
        if remaining > self.duration {
            return;
        }
        let t = 1.0 - remaining.div_duration_f32(self.duration);
        let t = SmoothStepIn::sample(t);
        text.position.offset.y -= t * self.max_offset;
        text.text_color = text.text_color.lerp_to_gamma(Color32::TRANSPARENT, t);
        if let Some(highlight_color) = text.highlight_color_mut() {
            *highlight_color = highlight_color.lerp_to_gamma(Color32::TRANSPARENT, t);
        }
    }
}
