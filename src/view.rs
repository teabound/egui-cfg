use std::collections::HashMap;

use crate::CfgLayout;
use crate::style::NodeStyle;
use crate::types::{BlockLike, EdgeKind, PortKind, PortLine, PortSlot};
use egui::epaint::CubicBezierShape;
use egui::{
    Align2, Color32, CornerRadius, Pos2, Rect, Shape, Stroke, StrokeKind, Ui, Vec2, pos2, vec2,
};
use petgraph::graph::NodeIndex;

pub struct CfgView<'a, N: BlockLike + Clone, E: Clone> {
    pub layout: &'a CfgLayout<N, E>,
    pub style: &'a NodeStyle,
    block_rects: HashMap<NodeIndex, Rect>,
    port_positions: HashMap<PortSlot, Pos2>,
    port_lines: Vec<PortLine>,
}

impl<'a, N: BlockLike + Clone, E: Clone> CfgView<'a, N, E> {
    pub fn new(layout: &'a CfgLayout<N, E>, style: &'a NodeStyle) -> Self {
        Self {
            layout,
            style,
            block_rects: HashMap::new(),
            port_lines: Vec::new(),
            port_positions: HashMap::new(),
        }
    }

    fn draw_block(&mut self, ui: &mut Ui, x: f32, y: f32, node: &NodeIndex) {
        // get the target basic block from the graph.
        let block = &self.layout.graph[*node];

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

        self.block_rects.insert(*node, block_rectangle);

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

        let text_pos = pos2(
            block_rectangle.min.x + style.padding.x,
            header_rectangle.max.y + style.padding.y,
        );

        ui.painter().galley(text_pos, body_galley, Color32::WHITE);
    }

    /// This will get the position at the point of a rect, either the top
    /// or bottom, where the next port should be placed depending on `count`.
    fn layout_ports_on_rect(rect: Rect, kind: PortKind, count: usize) -> Vec<Pos2> {
        let (left, right) = match kind {
            PortKind::Input => (rect.left_top(), rect.right_top()),
            PortKind::Output => (rect.left_bottom(), rect.right_bottom()),
        };

        (0..count)
            .map(|i| {
                let t = (i as f32 + 1.0) / (count as f32 + 1.0);
                left.lerp(right, t)
            })
            .collect()
    }

    fn draw_ports_for_node(
        &mut self,
        ui: &mut egui::Ui,
        node: NodeIndex,
        rect: Rect,
        inputs: usize,
        outputs: usize,
    ) {
        let mut draw_port = |pid: PortSlot, pos: Pos2| {
            let radius = self.style.edge.width * 3.0;

            // draw the first cricle, just the edge outline.
            ui.painter().circle_stroke(pos, radius, self.style.edge);

            // draw the second circle which is the filled "inside".
            ui.painter()
                .circle_filled(pos, radius - self.style.edge.width * 0.75, self.style.fill);

            self.port_positions.insert(pid, pos);
        };

        for (i, pos) in Self::layout_ports_on_rect(rect, PortKind::Input, inputs)
            .into_iter()
            .enumerate()
        {
            draw_port(PortSlot::new(node, i, PortKind::Input), pos);
        }

        for (i, pos) in Self::layout_ports_on_rect(rect, PortKind::Output, outputs)
            .into_iter()
            .enumerate()
        {
            draw_port(PortSlot::new(node, i, PortKind::Output), pos);
        }
    }

    pub fn show(&mut self, ui: &mut Ui, scene_rect: &mut Rect) {
        egui::Scene::new()
            .max_inner_size([
                self.layout.width as f32 + 800.0,
                self.layout.height as f32 + 800.0,
            ])
            .zoom_range(0.1..=2.0)
            .show(ui, scene_rect, |ui| {
                // this will draw the block in the egui ui panel, and also push the position on the
                // block rectangle to a hashmap, so that we can use it later.
                for (node, (x, y)) in &self.layout.coords {
                    self.draw_block(ui, *x as f32, *y as f32, node);
                }

                for node in self.layout.graph.node_indices() {
                    let graph = self.layout.graph.clone();

                    // get the indegree of hte current node.
                    let inputs = graph.neighbors_directed(node, petgraph::Incoming).count();
                    // get the outdegree of the current node.
                    let outputs = graph.neighbors_directed(node, petgraph::Outgoing).count();

                    if let Some(&rect) = self.block_rects.get(&node) {
                        self.draw_ports_for_node(ui, node, rect, inputs, outputs);
                    }
                }
            });
    }
}
