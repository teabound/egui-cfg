use eframe::{App, egui, glow::DEPTH_STENCIL};
use egui::{Color32, FontId, Pos2, Rect, Scene, Shape, Stroke, Vec2, pos2, vec2};
use petgraph::{
    graph::{EdgeIndex, NodeIndex},
    stable_graph::StableGraph,
    visit::{DfsEvent, NodeIndexable, depth_first_search},
};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum GraphLayoutError {
    #[error("Could not find entry point node in graph.")]
    NoEntryPoint,
    #[error("Could not find edge in graph.")]
    NoEdgeFound,
}

type Result<T> = std::result::Result<T, GraphLayoutError>;

#[derive(Clone, Debug)]
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
pub struct GraphLayout {
    graph: StableGraph<Block, BlockEdge>,
    /// The entry point node of the CFG.
    entry_node: Option<NodeIndex>,
    /// Back edges not picked up by the DFS.
    removed_edges: Vec<EdgeIndex>,
    /// Contains the row position a node is at.
    graph_row: Vec<usize>,
    /// All edges that are part of the DAG.
    target_edges: Vec<EdgeIndex>,
    /// Topologically sorted DAG nodes.
    sorted: Vec<NodeIndex>,
}

#[derive(Clone, Copy, PartialEq)]
enum EdgeStateDAG {
    NotVisited,
    InStack,
    Visited,
}

impl GraphLayout {
    fn new(graph: StableGraph<Block, BlockEdge>) -> Self {
        Self {
            graph_row: vec![0; graph.node_bound()],
            entry_node: None,
            removed_edges: Vec::new(),
            graph,
            target_edges: Vec::new(),
            sorted: Vec::new(),
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

    fn set_dag_edges_and_toposort(&mut self) -> Result<()> {
        // create a vector of node states, whether or not we've visited it yet, or whatever.
        // NOTE: not really needed here, maybe if we have disconnected nodes then revisit.
        // let mut state = vec![EdgeStateDAG::NotVisited; self.graph.node_bound()];

        let entry = self.get_entry_node()?;

        // this will contain the reverse topological order of the nodes.
        // a.k.a pushed to when we finish DFS a node.
        let mut reverse_order_nodes: Vec<NodeIndex> = Vec::new();

        // edges that make up valid DAG edges, i.e. don't contain cycle edges.
        let mut dag_edges: Vec<(NodeIndex, NodeIndex)> = Vec::new();

        depth_first_search(&self.graph, [entry], |event| match event {
            DfsEvent::Discover(_, _) => (),
            DfsEvent::TreeEdge(u, v) => dag_edges.push((u, v)),
            // NOTE: maybe push this edge to the removed edges member?
            DfsEvent::BackEdge(_, _) => (),
            DfsEvent::CrossForwardEdge(u, v) => dag_edges.push((u, v)),
            DfsEvent::Finish(n, _) => reverse_order_nodes.push(n),
        });

        // convert them into topological order.
        reverse_order_nodes.reverse();

        self.target_edges = dag_edges
            .iter()
            .map(|(u, v)| {
                // map them into edge indices, rather than keeping them "pseudo" edges.
                self.graph
                    .find_edge(*u, *v)
                    .ok_or(GraphLayoutError::NoEdgeFound)
            })
            .collect::<Result<_>>()?;

        self.sorted = reverse_order_nodes;

        Ok(())
    }

    fn assign_rows(&mut self) -> Result<()> {
        let (edges, sorted) = (&self.target_edges, &self.sorted);

        for &u in sorted.iter() {
            let base_node_value = self.graph_row[u.index()];

            // filter through the edges whose source is `u`.
            for (_, dst) in edges
                .iter()
                .map(|&e| self.graph.edge_endpoints(e).unwrap())
                .filter(|(s, _)| *s == u)
                .collect::<Vec<(NodeIndex, NodeIndex)>>()
            {
                // make it so that we increase all outward nodes' row value, but we also protect
                // against forward and and cross edges by getting the max of its value or the "new" value.
                self.graph_row[dst.index()] = self.graph_row[dst.index()].max(base_node_value + 1);
            }
        }

        Ok(())
    }

    fn select_tree(&mut self) -> Result<Vec<EdgeIndex>> {
        // keep track of whether the node has a parent in an upper level/row/layer.
        let mut has_parent = vec![false; self.graph.node_bound()];

        // collect spanning tree edges.
        let mut tree = Vec::new();

        for &edge in self.target_edges.iter() {
            let (u, v) = self
                .graph
                .edge_endpoints(edge)
                .ok_or(GraphLayoutError::NoEdgeFound)?;

            // take edge only if it goes one row downward and the child has no parent yet.
            if self.graph_row[u.index()] + 1 == self.graph_row[v.index()] && !has_parent[v.index()]
            {
                tree.push(edge);
                has_parent[v.index()] = true;
            }
        }

        Ok(tree)
    }
}

fn main() -> eframe::Result<()> {
    let mut graph_layout = GraphLayout::new(build_dummy_cfg());

    graph_layout.set_dag_edges_and_toposort().unwrap();
    graph_layout.assign_rows().unwrap();
    graph_layout.select_tree().unwrap();

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
