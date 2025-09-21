use std::collections::HashMap;

use crate::BlockLike;
use crate::CfgLayout;
use crate::LayoutConfig;
use crate::get_cfg_layout;
use crate::route::{AStar, CostField, Grid};
use crate::style::NodeStyle;
use egui::{
    Align2, Color32, CornerRadius, Pos2, Rect, Shape, Stroke, StrokeKind, Ui, Vec2, pos2, vec2,
};
use petgraph::graph::NodeIndex;
use petgraph::prelude::StableGraph;
use petgraph::visit::EdgeRef;

/// The offset from the port to the basic block rectangle.
const PORT_OFFSET: f32 = 4.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PortKind {
    Input,
    Output,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PortSlot {
    pub node: NodeIndex,
    pub slot: usize,
    pub kind: PortKind,
}

impl PortSlot {
    pub fn new(node: NodeIndex, slot: usize, kind: PortKind) -> Self {
        Self { node, slot, kind }
    }
}

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct PortLine {
    pub from: PortSlot,
    pub to: PortSlot,
}

pub struct CfgView<'a, N: BlockLike + Clone, E: Clone> {
    graph: StableGraph<N, E>,
    layout_config: LayoutConfig,
    block_rects: HashMap<NodeIndex, Rect>,
    port_positions: HashMap<PortSlot, Pos2>,
    port_lines: Vec<PortLine>,
    pub style: &'a NodeStyle,
}

impl<'a, N: BlockLike + Clone, E: Clone> CfgView<'a, N, E> {
    pub fn new(graph: StableGraph<N, E>, config: LayoutConfig, style: &'a NodeStyle) -> Self {
        Self {
            graph,
            layout_config: config,
            style,
            block_rects: HashMap::new(),
            port_lines: Vec::new(),
            port_positions: HashMap::new(),
        }
    }

    /// Get a rectangle the encompasses every block node placed.
    fn get_world_rect(&self, expand: Option<f32>) -> Rect {
        let mut bounds = egui::Rect::NOTHING;

        // unionize all of the rects we created.
        for rects in self.block_rects.values() {
            bounds = bounds.union(*rects);
        }

        bounds.expand(expand.unwrap_or(100.0))
    }

    /// This will draw blocks in the egui ui panel, and also push the position on the
    /// block rectangle to a hashmap, so that we can use it later.
    fn assign_and_draw_blocks(&mut self, ui: &mut Ui, layout: &CfgLayout) {
        for (node, coords) in &layout.coords {
            let (x, y) = (coords.0 as f32, coords.1 as f32);

            // get the target basic block from the graph.
            let block = &self.graph[*node];

            let style = self.style;

            let (mut block_rectangle, body_galley) = crate::get_block_rectangle(ui, block, style);

            // give the rectangle the correct position.
            block_rectangle.set_center(Pos2::new(x, y));

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

    fn assign_port_positions(&mut self) {
        for node in self.graph.node_indices() {
            let graph = &self.graph;

            // get the indegree of hte current node.
            let inputs = graph.neighbors_directed(node, petgraph::Incoming).count();
            // get the outdegree of the current node.
            let outputs = graph.neighbors_directed(node, petgraph::Outgoing).count();

            if let Some(&rect) = self.block_rects.get(&node) {
                for (i, mut pos) in Self::layout_ports_on_rect(rect, PortKind::Input, inputs)
                    .into_iter()
                    .enumerate()
                {
                    let port = PortSlot::new(node, i, PortKind::Input);
                    // we offset so the ports don't overlap with the basic block rectangles.
                    pos.y += -PORT_OFFSET;
                    self.port_positions.insert(port, pos);
                }

                for (i, mut pos) in Self::layout_ports_on_rect(rect, PortKind::Output, outputs)
                    .into_iter()
                    .enumerate()
                {
                    let port = PortSlot::new(node, i, PortKind::Output);
                    // we offset so the ports don't overlap with the basic block rectangles.
                    pos.y += PORT_OFFSET;
                    self.port_positions.insert(port, pos);
                }
            }
        }
    }

    fn draw_arrow_tip(&self, ui: &mut egui::Ui, tip: egui::Pos2, dir: Option<egui::Vec2>) {
        let size = self.style.edge.width * 4.0;

        // get the unit direction of the arrow
        let dir = dir.unwrap_or(egui::vec2(0.0, 1.0)).normalized();

        // get the base of the triangle.
        let base = tip - dir * size;

        // set the vector perpendicular to tip->base, half the size of the base.
        let perp = egui::vec2(-dir.y, dir.x) * (size * 0.5);

        let p1 = base + perp;
        let p2 = base - perp;

        ui.painter().add(egui::Shape::convex_polygon(
            vec![tip, p1, p2],
            self.style.edge.color,
            self.style.edge,
        ));
    }

    fn draw_ports(&mut self, ui: &mut egui::Ui) {
        for (slot, mut pos) in self.port_positions.clone() {
            match slot.kind {
                PortKind::Output => {
                    // draw the port closer to the block.
                    pos.y -= PORT_OFFSET - 2.0;

                    let radius = self.style.edge.width * 3.0;

                    ui.painter().circle_stroke(pos, radius, self.style.edge);
                    ui.painter().circle_filled(pos, radius, self.style.fill);
                }

                PortKind::Input => {
                    // draw the port closer to the block.
                    pos.y += PORT_OFFSET;

                    self.draw_arrow_tip(ui, pos, None);
                }
            }
        }
    }

    /// This will assign a port "edge", from one port to another.
    ///
    /// For every node in the in the graph connect each outgoing to port to an incoming port.
    fn assign_port_lines(&mut self) {
        let center_x = |n: NodeIndex| {
            self.block_rects
                .get(&n)
                .map(|r| r.center().x)
                .unwrap_or(0.0)
        };

        let sorted_ports = |node: NodeIndex, kind: PortKind| -> Vec<PortSlot> {
            let mut target_ports: Vec<(PortSlot, f32)> = self
                .port_positions
                .iter()
                .filter_map(|(slot, pos)| {
                    // get the target slots from the target node, which are `kind`.
                    (slot.node == node && slot.kind == kind).then_some((*slot, pos.x))
                })
                .collect();

            // yes, it's super weird to sort f32s, but whatever.
            target_ports.sort_by(|a, b| a.1.total_cmp(&b.1));

            target_ports.into_iter().map(|(slot, _)| slot).collect()
        };

        for node in self.graph.node_indices() {
            let ports = sorted_ports(node, PortKind::Output);

            if ports.is_empty() {
                continue;
            }

            let mut sorted_out_edges: Vec<(petgraph::graph::EdgeIndex, NodeIndex)> = self
                .graph
                .edges_directed(node, petgraph::Direction::Outgoing)
                .map(|e| (e.id(), e.target()))
                .collect();

            if sorted_out_edges.is_empty() {
                continue;
            }

            // yes, it's super weird to sort f32s, but whatever.
            sorted_out_edges
                .sort_by(|(_, lhs), (_, rhs)| center_x(*lhs).total_cmp(&center_x(*rhs)));

            // we associate each outgoing edge with an outgoing port.
            for (n, (edge, target_node)) in sorted_out_edges.iter().enumerate() {
                let Some(&from_port) = ports.get(n) else {
                    continue;
                };

                let target_ports = sorted_ports(*target_node, PortKind::Input);

                if target_ports.is_empty() {
                    continue;
                }

                // collect and sort the incoming edges from the target node.
                let mut incoming: Vec<(petgraph::graph::EdgeIndex, NodeIndex)> = self
                    .graph
                    .edges_directed(*target_node, petgraph::Direction::Incoming)
                    .map(|e| (e.id(), e.source()))
                    .collect();

                incoming.sort_by(|(_, lhs), (_, rhs)| center_x(*lhs).total_cmp(&center_x(*rhs)));

                // we want to get the port offset at the same index of the edge.
                let target_port = incoming.iter().position(|(e, _)| e == edge).unwrap_or(0);

                let Some(&to_port) = target_ports.get(target_port) else {
                    continue;
                };

                self.port_lines.push(PortLine {
                    from: from_port,
                    to: to_port,
                });
            }
        }
    }

    fn build_field(&self, scene: egui::Rect) -> CostField {
        let grid = Grid::from_scene(scene, 3.0);

        let mut field = CostField::new(grid);

        // we just want to hard block pathfinding from going through block rects.
        for rect in self.block_rects.values() {
            field.add_block_rect(*rect, 5.0);
        }

        field
    }

    fn draw_edges(&mut self, ui: &mut egui::Ui, scene_rect: egui::Rect) {
        let mut field = self.build_field(scene_rect);

        let id = ui.make_persistent_id("cfg_edge_cache_v1");
        let mut routed_polylines: Vec<Vec<egui::Pos2>> = Vec::new();

        if let Some(lines) = ui
            .ctx()
            .data_mut(|d| d.get_persisted::<Vec<Vec<egui::Pos2>>>(id))
        {
            for poly in lines {
                ui.painter()
                    .add(egui::Shape::line(poly.clone(), self.style.edge));
            }
            return;
        }

        for pl in &self.port_lines {
            let Some(&from) = self.port_positions.get(&pl.from) else {
                continue;
            };

            let Some(&to) = self.port_positions.get(&pl.to) else {
                continue;
            };

            let mut astar = AStar::new(&field);

            if let Some(poly) = astar.find_path(from, to) {
                routed_polylines.push(poly);
            }
        }

        for poly in routed_polylines.iter() {
            ui.painter()
                .add(egui::Shape::line(poly.clone(), self.style.edge));
        }

        ui.ctx()
            .data_mut(|d| d.insert_persisted(id, routed_polylines));
    }

    pub fn show(&mut self, ui: &mut Ui, scene_rect: &mut Rect) {
        // calculate the layout of the graph.
        // btw this should be pretty cheap to calculate.
        let layout = get_cfg_layout(ui, &self.graph, &self.layout_config, &self.style);

        egui::Scene::new()
            .max_inner_size([layout.width as f32 + 800.0, layout.height as f32 + 800.0])
            .zoom_range(0.1..=2.0)
            .show(ui, scene_rect, |ui| {
                self.assign_and_draw_blocks(ui, &layout);
                self.assign_port_positions();
                self.assign_port_lines();
                self.draw_edges(ui, self.get_world_rect(None));
                self.draw_ports(ui);
            });
    }
}
