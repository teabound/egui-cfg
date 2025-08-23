use eframe::{App, egui};
use egui::{
    Align2, Color32, CornerRadius, FontId, Pos2, Rect, Scene, Shape, Stroke, Vec2, pos2, vec2,
};
use egui_graphs::{DrawContext, SettingsInteraction, SettingsNavigation, SettingsStyle};
use petgraph::{
    algo::is_cyclic_directed,
    graph::{EdgeIndex, NodeIndex},
    stable_graph::StableGraph,
    visit::EdgeRef,
};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum GraphLayoutError {
    #[error("Could not find entry point node in graph.")]
    NoEntryPoint,
}

type Result<T> = std::result::Result<T, GraphLayoutError>;

#[derive(Clone)]
struct Block {
    addr: u64,
    name: &'static str,
    code: &'static [&'static str],
    entry: bool,
    exit: bool,
}

#[derive(Clone)]
enum BlockEdge {
    FT,
    Jmp,
    Taken,
}

pub struct NodeStyle {
    pub size: egui::Vec2,
    pub padding: egui::Vec2,
    pub rounding: u8,
    pub fill: Color32,
    pub header_fill: Color32,
    pub stroke: Stroke,
    pub header_h: f32,
    pub label_font: FontId,
    pub text_font: FontId,

    pub edge: Stroke,
    pub arrow_len: f32,
    pub arrow_w: f32,

    pub side_lane: f32,
}

impl Default for NodeStyle {
    fn default() -> Self {
        Self {
            size: Vec2::new(240.0, 100.0),
            padding: Vec2::new(8.0, 6.0),
            rounding: 8,
            fill: Color32::from_rgb(27, 31, 39),
            header_fill: Color32::from_rgb(40, 44, 54),
            stroke: Stroke::new(1.0, Color32::from_gray(70)),
            header_h: 22.0,
            label_font: FontId::monospace(12.0),
            text_font: FontId::monospace(12.0),
            edge: Stroke::new(1.8, Color32::from_rgb(210, 210, 230)),
            arrow_len: 10.0,
            arrow_w: 7.0,
            side_lane: 120.0,
        }
    }
}

fn build_dummy_cfg() -> StableGraph<Block, BlockEdge> {
    let mut g: StableGraph<Block, BlockEdge> = StableGraph::new();

    let entry = g.add_node(Block {
        addr: 0x1000,
        name: "entry",
        code: &["push rbp", "mov  rbp, rsp"],
        entry: true,
        exit: false,
    });

    let cond = g.add_node(Block {
        addr: 0x1005,
        name: "cmp_and_branch",
        code: &["cmp  rdi, 0", "jl   then_else"],
        entry: false,
        exit: false,
    });

    let then_ = g.add_node(Block {
        addr: 0x1010,
        name: "then",
        code: &["neg  rdi", "mov  rax, rdi"],
        entry: false,
        exit: false,
    });

    let else_ = g.add_node(Block {
        addr: 0x1018,
        name: "else",
        code: &["mov  rax, rdi"],
        entry: false,
        exit: false,
    });

    let exit = g.add_node(Block {
        addr: 0x1020,
        name: "exit",
        code: &["pop  rbp", "ret"],
        entry: false,
        exit: true,
    });

    g.add_edge(entry, cond, BlockEdge::FT);
    g.add_edge(cond, then_, BlockEdge::Taken);
    g.add_edge(cond, else_, BlockEdge::FT);
    g.add_edge(then_, exit, BlockEdge::Jmp);
    g.add_edge(else_, exit, BlockEdge::Jmp);

    g
}

struct MyApp {
    scene_rect: Rect,
    gl: GraphLayout,
}

#[derive(Clone)]
struct GraphLayout {
    graph: StableGraph<Block, BlockEdge>,
    entry_node: Option<NodeIndex>,
    removed_edges: Vec<EdgeIndex>,
}

impl GraphLayout {
    fn new(graph: StableGraph<Block, BlockEdge>) -> Self {
        Self {
            graph,
            entry_node: None,
            removed_edges: Vec::new(),
        }
    }

    fn get_entry_node(&mut self) -> Result<NodeIndex> {
        // if we've already found the entry then return it.
        if let Some(i) = self.entry_node {
            return Ok(i);
        }

        // find the node that is marked as the entry point.
        let node = self
            .graph
            .node_indices()
            .find(|&i| self.graph[i].entry)
            .ok_or(GraphLayoutError::NoEntryPoint)?;

        // store the node marked as the entry point.
        self.entry_node = Some(node);

        Ok(node)
    }

    fn remove_cycles(&mut self) -> Result<()> {
        // if the graph is acyclical then we don't need to remove cycles.
        if !is_cyclic_directed(&self.graph) {
            return Ok(());
        }

        // get the feedback arc set from the graph, so that we can remove the cycle creating edges.
        let fas: Vec<EdgeIndex> = petgraph::algo::greedy_feedback_arc_set(&self.graph)
            .map(|e| e.id())
            .collect();

        // remove every edge that causes a cycle in the graph.
        fas.iter().for_each(|&e| {
            self.graph.remove_edge(e);
        });

        self.removed_edges = fas;

        Ok(())
    }
}

fn main() -> eframe::Result<()> {
    let mut graph_layout = GraphLayout::new(build_dummy_cfg());

    // remove cycles.
    graph_layout.remove_cycles().unwrap();

    eframe::run_native(
        "CFG",
        eframe::NativeOptions::default(),
        Box::new(|_cc| {
            Ok(Box::new(MyApp {
                scene_rect: Rect::from_min_size(pos2(-1000.0, -1000.0), vec2(3000.0, 2000.0)),
                gl: graph_layout,
            }))
        }),
    )
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            Scene::new()
                .max_inner_size([350.0, 1000.0])
                .zoom_range(0.1..=2.0)
                .show(ui, &mut self.scene_rect, |ui| {
                    ui.label("test");
                });
        });
    }
}

