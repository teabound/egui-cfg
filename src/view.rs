use crate::CfgLayout;
use crate::style::NodeStyle;
use crate::types::{BlockLike, EdgeKind};
use egui::{Align2, Color32, CornerRadius, Pos2, Rect, Stroke, StrokeKind, Ui, Vec2, pos2, vec2};
use petgraph::stable_graph::StableGraph;

pub struct CfgView<'a, N: BlockLike, E: Clone> {
    pub graph: &'a StableGraph<N, E>,
    pub layout: &'a CfgLayout,
    pub style: &'a NodeStyle,
}

impl<'a, N: BlockLike, E: Clone> CfgView<'a, N, E> {
    pub fn show(self, ui: &mut Ui, scene_rect: &mut Rect) {
        egui::Scene::new()
            .max_inner_size([
                self.layout.width as f32 + 800.0,
                self.layout.height as f32 + 800.0,
            ])
            .zoom_range(0.1..=2.0)
            .show(ui, scene_rect, |ui| {
                for (node, (x, y)) in &self.layout.coords {
                    let n = &self.graph[*node];
                    self.draw_block(ui, *x as f32, *y as f32, n);
                }
            });
    }

    fn draw_block(&self, ui: &mut Ui, x: f32, y: f32, block: &N) {
        let style = self.style;

        // where the block that we're going to draw starts.
        let block_position = ui.min_rect().min + Vec2::new(x, y);

        // get the width of the content (the size of the node without the padding).
        let content_width = style.size.x - style.padding.x * 2.0;

        let body_text = block.body_lines().join("\n");

        // get the text galley so we can get information related to it.
        let body_galley = ui.fonts(|f| {
            f.layout(
                body_text,
                style.text_font.clone(),
                Color32::WHITE,
                content_width,
            )
        });

        // ge the total size of the height including the padding, the text and the header.
        let block_height = style.header_height + style.padding.y * 2.0 + body_galley.size().y;

        // create a rectangle starting from the start of our block and is the size we've calculated
        // from the content in the block.
        let block_rectangle = Rect::from_min_size(block_position, vec2(style.size.x, block_height));

        let corner_rounding = CornerRadius::same(style.rounding);

        // draw the entire node block.
        ui.painter().rect(
            block_rectangle,
            corner_rounding,
            style.fill,
            style.stroke,
            StrokeKind::Inside,
        );

        // the header rectangle, width is the size of the block, then we just add the header height.
        let header_rectangle = Rect::from_min_max(
            block_rectangle.min,
            pos2(
                block_rectangle.max.x,
                block_rectangle.min.y + style.header_height,
            ),
        );

        ui.painter().rect(
            header_rectangle,
            CornerRadius {
                nw: style.rounding,
                ne: style.rounding,
                se: 0,
                sw: 0,
            },
            style.header_fill,
            Stroke::NONE,
            StrokeKind::Inside,
        );

        // block title, could be empty or not.
        let label = format!("{}", block.title());
        // NOTE: have an option to put the title in the middle of the header rectangle.
        let label_pos = header_rectangle.left_center() + vec2(style.padding.x, 0.0);

        ui.painter().text(
            label_pos,
            Align2::LEFT_CENTER,
            label,
            style.label_font.clone(),
            Color32::WHITE,
        );

        // Body text
        let text_pos = pos2(
            block_rectangle.min.x + style.padding.x,
            header_rectangle.max.y + style.padding.y,
        );

        ui.painter().galley(text_pos, body_galley, Color32::WHITE);
    }
}
