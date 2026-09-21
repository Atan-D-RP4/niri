use niri_config::BackgroundEffectKind;

use crate::render_helpers::blur::Blur;
use crate::render_helpers::glass::GlassEffect;
use crate::render_helpers::glass::GlassEffectParams;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EffectParams {
    pub noise: f32,
    pub saturation: f32,
    pub pointer: Option<(f32, f32)>,
    pub time: f32,
}

pub type GlassOptions = GlassEffectParams;

#[derive(Debug)]
pub enum EffectImpl {
    Blur(Blur),
    Glass(GlassEffect),
}

pub trait Effect: std::fmt::Debug {
    fn kind(&self) -> BackgroundEffectKind;

    fn damage(&mut self);

    fn needs_continuous_damage(&self) -> bool;

    fn has_custom_shader(&self) -> bool {
        false
    }
}
