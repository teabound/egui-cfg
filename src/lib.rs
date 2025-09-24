pub mod route;
pub mod style;
pub mod view;

use crate::style::NodeStyle;
use egui::{Color32, Galley, Pos2, Rect, Ui, vec2};
use petgraph::{
    graph::NodeIndex,
    stable_graph::StableGraph,
    visit::{EdgeRef, IntoEdgeReferences},
};

pub trait BlockLike: Clone {
    fn title(&self) -> &str;
    fn body_lines(&self) -> &[String];
}

#[derive(Clone, Debug, Copy)]
pub enum EdgeKind {
    Taken,
    FallThrough,
    Unconditional,
}

pub trait EdgeLike: Clone {
    fn kind(&self) -> EdgeKind;
}

impl EdgeLike for EdgeKind {
    fn kind(&self) -> EdgeKind {
        *self
    }
}

#[derive(Clone, Debug, Default)]
pub struct CfgLayout {
    pub coords: Vec<(NodeIndex, (f64, f64))>,
    pub width: f64,
    pub height: f64,
}

#[derive(Clone, Debug)]
pub struct LayoutConfig {
    pub vertex_spacing: f64,
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self {
            vertex_spacing: 30.0,
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

pub fn get_block_rectangle<N: BlockLike>(
    ui: &Ui,
    block: &N,
    style: &NodeStyle,
) -> (Rect, std::sync::Arc<Galley>) {
    // where the block that we're going to draw starts.
    let block_position = Pos2::new(0.0, 0.0);

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
    let rect = Rect::from_min_size(block_position, vec2(style.size.x, block_height));

    (rect, body_galley)
}

pub fn get_cfg_layout<N: BlockLike, E: Clone>(
    ui: &Ui,
    graph: &StableGraph<N, E>,
    config: &LayoutConfig,
    style: &NodeStyle,
) -> CfgLayout {
    // Get the block rectangle to use as the vertex size.
    let vertex_size = |_: NodeIndex, n: &N| {
        let rect = get_block_rectangle(ui, n, style).0;
        (rect.width() as _, rect.height() as f64)
    };

    let mut graph = graph.clone();

    // get all nodes that have an outgoing edge that connects to the same node.
    let loops: Vec<(petgraph::graph::NodeIndex, E)> = graph
        .edge_references()
        .filter(|e| e.source() == e.target())
        .map(|e| (e.source(), e.weight().clone()))
        .collect();

    // remove all the edges that point to the same node.
    for edge in graph.edge_indices().collect::<Vec<_>>() {
        if let Some((u, v)) = graph.edge_endpoints(edge) {
            if u == v {
                graph.remove_edge(edge);
            }
        }
    }

    let info = rust_sugiyama::from_graph(&graph, &vertex_size, &config.into());

    // NOTE: maybe there will be a case when we need to get the full vector.
    let (coords, width, height) = info[0].clone();

    // add the loop back after we've placed nodes.
    for (n, w) in loops {
        graph.add_edge(n, n, w);
    }

    CfgLayout {
        coords,
        width,
        height,
    }
}
