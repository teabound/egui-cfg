use cfg::{BlockLike, EdgeKind, LayoutConfig, style::NodeStyle, view::CfgView};

use eframe::egui::{self, Rect, pos2, vec2};
use eframe::{self};
use petgraph::graph::NodeIndex;
use petgraph::stable_graph::StableGraph;

#[derive(Clone, Debug)]
struct BasicBlock {
    addr: u64,
    title: String,
    code: Vec<String>,
}

impl BlockLike for BasicBlock {
    fn title(&self) -> &str {
        &self.title
    }

    fn body_lines(&self) -> &[String] {
        &self.code
    }
}

fn build_dummy_cfg() -> StableGraph<BasicBlock, EdgeKind> {
    let mut g = StableGraph::new();

    let entry = g.add_node(BasicBlock {
        addr: 0x1000,
        title: "entry".into(),
        code: vec!["push rbp".into(), "mov rbp, rsp".into()],
    });

    let cond = g.add_node(BasicBlock {
        addr: 0x1005,
        title: "cmp and branch".into(),
        code: vec!["cmp rdi, 0".into(), "jl then_else".into()],
    });

    let then_ = g.add_node(BasicBlock {
        addr: 0x1010,
        title: "then".into(),
        code: vec!["neg rdi".into(), "mov rax, rdi".into()],
    });

    let else_ = g.add_node(BasicBlock {
        addr: 0x1018,
        title: "else".into(),
        code: vec!["mov rax, rdi".into()],
    });

    let exit = g.add_node(BasicBlock {
        addr: 0x1020,
        title: "exit".into(),
        code: vec!["pop rbp".into(), "ret".into()],
    });

    g.add_edge(entry, cond, EdgeKind::FallThrough);
    g.add_edge(cond, then_, EdgeKind::Taken);
    g.add_edge(cond, else_, EdgeKind::FallThrough);
    g.add_edge(then_, exit, EdgeKind::Unconditional);
    g.add_edge(else_, exit, EdgeKind::Unconditional);

    g
}

struct App {
    graph: StableGraph<BasicBlock, EdgeKind>,
    selected: Option<NodeIndex>,
    style: NodeStyle,
    scene_rect: Rect,
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            CfgView::new(
                self.graph.clone(),
                LayoutConfig::default(),
                &mut self.selected,
                &self.style,
            )
            .show(ui, &mut self.scene_rect);
        });
    }
}

fn main() -> eframe::Result<()> {
    let scene_rect = Rect::from_min_size(pos2(-1000.0, -1000.0), vec2(2000.0, 2000.0));

    eframe::run_native(
        "CFG Demo",
        eframe::NativeOptions::default(),
        Box::new(|_| {
            Ok(Box::new(App {
                selected: None,
                graph: build_dummy_cfg(),
                style: NodeStyle::default(),
                scene_rect,
            }))
        }),
    )
}
