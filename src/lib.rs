pub mod route;
pub mod style;
pub mod view;

use crate::style::{NodeStyle, approx_block_height};
use egui::Rect;
use petgraph::{graph::NodeIndex, stable_graph::StableGraph};

pub trait BlockLike {
    fn title(&self) -> &str;
    fn body_lines(&self) -> &[String];
    fn is_entry(&self) -> bool {
        false
    }
    fn is_exit(&self) -> bool {
        false
    }
}

#[derive(Clone, Debug)]
pub enum EdgeKind {
    Taken,
    FallThrough,
    Unconditional,
}

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

#[derive(Clone, Debug)]
pub struct CfgLayout<N: BlockLike + Clone, E: Clone> {
    pub coords: Vec<(NodeIndex, (f64, f64))>,
    pub width: f64,
    pub height: f64,
    pub graph: StableGraph<N, E>,
}

#[derive(Clone, Debug)]
pub struct LayoutConfig {
    pub vertex_spacing: f64,
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self {
            vertex_spacing: 5.0,
        }
    }
}

impl From<&LayoutConfig> for rust_sugiyama::configure::Config {
    fn from(lhs: &LayoutConfig) -> Self {
        let mut cfg = rust_sugiyama::configure::Config::default();

        cfg.vertex_spacing = lhs.vertex_spacing;

        cfg
    }
}

pub fn layout_graph<N: BlockLike + Clone, E: Clone>(
    graph: &StableGraph<N, E>,
    style: &NodeStyle,
    config: &LayoutConfig,
) -> CfgLayout<N, E> {
    // we approximate the size of the node so sugiyama can place vertices accordingly.
    let vertex_size = |_: NodeIndex, n: &N| {
        let w = style.size.x as f64;
        let h = approx_block_height(n, style) as f64;
        (w, h)
    };

    let info = rust_sugiyama::from_graph(graph, &vertex_size, &config.into());

    // NOTE: maybe there will be a case when we need to get the full vector.
    let (coords, width, height) = info[0].clone();

    CfgLayout {
        coords,
        width,
        height,
        graph: graph.clone(),
    }
}
