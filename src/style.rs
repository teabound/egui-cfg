use egui::{self, Color32, FontId, Stroke, TextStyle, Vec2, vec2};

use crate::BlockLike;

/// This is the style of the Basic Block graph node.
///
/// a.k.a how it actaully appears when rendered.
#[derive(Clone)]
pub struct NodeStyle {
    /// The size of the whole node rectangle.
    pub size: egui::Vec2,
    /// The n,w,e,s padding inside of the node.
    pub padding: egui::Vec2,
    pub button_padding: egui::Vec2,
    pub rounding: u8,
    pub fill: Color32,
    pub header_fill: Color32,
    pub stroke: Stroke,
    /// The height of the header, or title box.
    pub header_height: f32,
    pub label_font: FontId,
    pub text_font: FontId,
    pub edge: Stroke,
    pub arrow_len: f32,
    pub arrow_w: f32,
    pub select: Stroke,
    pub select_bg: Color32,
}

impl NodeStyle {
    pub fn from_style(style: &egui::Style) -> Self {
        let mono = style
            .text_styles
            .get(&TextStyle::Monospace)
            .cloned()
            .unwrap_or(FontId::monospace(12.0));

        let body = style
            .text_styles
            .get(&TextStyle::Body)
            .cloned()
            .unwrap_or(FontId::proportional(12.0));

        let visuals = &style.visuals;
        let non_interactive = &visuals.widgets.noninteractive;
        let inactive = &visuals.widgets.inactive;
        let spacing = &style.spacing;

        Self {
            size: vec2(260.0, 120.0),
            padding: Vec2::new(10.0, 10.0),
            button_padding: spacing.button_padding,
            rounding: non_interactive.rounding().nw,
            fill: visuals.code_bg_color,
            header_fill: inactive.bg_fill,
            stroke: non_interactive.bg_stroke,
            header_height: spacing.interact_size.y,
            label_font: mono.clone(),
            text_font: mono,
            edge: non_interactive.fg_stroke,
            arrow_len: spacing.icon_width,
            arrow_w: spacing.icon_width_inner,
            select: style.visuals.selection.stroke,
            select_bg: style.visuals.selection.bg_fill,
        }
    }
}

impl Default for NodeStyle {
    fn default() -> Self {
        Self::from_style(&egui::Style::default())
    }
}
