use cfg::{
    CfgLayout, LayoutConfig, layout_graph, style::NodeStyle, types::BlockLike, types::EdgeKind,
    view::CfgView,
};

use eframe::egui::{self, Rect, pos2, vec2};
use eframe::{self};
use petgraph::stable_graph::StableGraph;

#[derive(Clone, Debug)]
struct BasicBlock {
    addr: u64,
    name: String,
    code: Vec<String>,
    entry: bool,
    exit: bool,
}

impl BlockLike for BasicBlock {
    fn title(&self) -> &str {
        &self.name
    }
    fn body_lines(&self) -> &[String] {
        &self.code
    }
    fn is_entry(&self) -> bool {
        self.entry
    }
    fn is_exit(&self) -> bool {
        self.exit
    }
}

fn build_dummy_cfg() -> StableGraph<BasicBlock, EdgeKind> {
    let mut g = StableGraph::new();
    let entry = g.add_node(BasicBlock {
        addr: 0x1000,
        name: "entry".into(),
        code: vec!["push rbp".into(), "mov rbp, rsp".into()],
        entry: true,
        exit: false,
    });
    let cond = g.add_node(BasicBlock {
        addr: 0x1005,
        name: "cmp_and_branch".into(),
        code: vec!["cmp rdi, 0".into(), "jl then_else".into()],
        entry: false,
        exit: false,
    });
    let then_ = g.add_node(BasicBlock {
        addr: 0x1010,
        name: "then".into(),
        code: vec!["neg rdi".into(), "mov rax, rdi".into()],
        entry: false,
        exit: false,
    });
    let else_ = g.add_node(BasicBlock {
        addr: 0x1018,
        name: "else".into(),
        code: vec!["mov rax, rdi".into()],
        entry: false,
        exit: false,
    });
    let exit = g.add_node(BasicBlock {
        addr: 0x1020,
        name: "exit".into(),
        code: vec!["pop rbp".into(), "ret".into()],
        entry: false,
        exit: true,
    });

    g.add_edge(entry, cond, EdgeKind::FallThrough);
    g.add_edge(cond, then_, EdgeKind::Taken);
    g.add_edge(cond, else_, EdgeKind::FallThrough);
    g.add_edge(then_, exit, EdgeKind::Unconditional);
    g.add_edge(else_, exit, EdgeKind::Unconditional);
    g.add_edge(exit, entry, EdgeKind::Unconditional);
    g
}

struct App {
    layout: CfgLayout<BasicBlock, EdgeKind>,
    style: NodeStyle,
    scene_rect: Rect,
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            CfgView::new(&self.layout, &self.style).show(ui, &mut self.scene_rect);
        });
    }
}

fn main() -> eframe::Result<()> {
    let style = NodeStyle::default();
    let graph = build_dummy_cfg();
    let layout = layout_graph(
        &graph,
        &style,
        &LayoutConfig {
            vertex_spacing: 30.0,
        },
    );

    let scene_rect = Rect::from_min_size(pos2(-1000.0, -1000.0), vec2(2000.0, 2000.0));

    eframe::run_native(
        "CFG Demo",
        eframe::NativeOptions::default(),
        Box::new(|_| {
            Ok(Box::new(App {
                layout,
                style,
                scene_rect,
            }))
        }),
    )
}
