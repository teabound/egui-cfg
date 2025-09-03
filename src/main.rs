use eframe::egui;
use egui::{
    Align2, Color32, CornerRadius, FontId, Pos2, Rect, Rounding, Scene, Shape, Stroke, StrokeKind,
    TextStyle, Vec2, pos2, vec2,
};
use petgraph::{
    graph::{EdgeIndex, NodeIndex},
    stable_graph::StableGraph,
    visit::{DfsEvent, NodeIndexable, depth_first_search},
};
use rust_sugiyama::configure::Config;
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
    Taken,
    FallThrough,
    Unconditional,
}

#[derive(Clone)]
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

        // code block bg or noninteractive bg.
        let fill = visuals.code_bg_color;

        // header a tad stronger.
        let header_fill = inactive.bg_fill;

        // stroke noninteractive outline/fg.
        let stroke = non_interactive.bg_stroke;

        // rounding and paddings from style.
        let rounding = non_interactive.rounding().nw;
        let padding = spacing.button_padding;

        // use default height of button, and other widgets for the header.
        let header_height = spacing.interact_size.y;

        // arrow head stuff, which we might not use.
        let arrow_len = spacing.icon_width;
        let arrow_w = spacing.icon_width_inner;

        // just give this a magic value for now.
        let side_lane = spacing.indent * 3.0;

        // NOTE: we'll set this to some dummy value for now and just change it later.
        let size = vec2(260.0, 120.0);

        Self {
            size,
            padding,
            rounding,
            fill,
            header_fill,
            stroke,
            header_h: header_height,
            label_font: mono.clone(),
            text_font: mono, // monospace body for code
            edge: non_interactive.fg_stroke,
            arrow_len,
            arrow_w,
            side_lane,
        }
    }
}

impl Default for NodeStyle {
    fn default() -> Self {
        Self::from_style(&egui::Style::default())
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

    g.add_edge(entry, cond, BlockEdge::FallThrough);
    g.add_edge(cond, then_, BlockEdge::Taken);
    g.add_edge(cond, else_, BlockEdge::FallThrough);
    g.add_edge(then_, exit, BlockEdge::Unconditional);
    g.add_edge(else_, exit, BlockEdge::Unconditional);

    g
}

fn approx_block_height(block: &Block, style: &NodeStyle) -> f32 {
    // crude but effective: monospace lines ~= 1.25 * point size
    let line_h = style.text_font.size * 1.25;
    let body_h = (block.code.len() as f32) * line_h;
    style.header_h + style.padding.y + body_h + style.padding.y
}

struct GraphLayout {
    /// Contain the coordinates for the node, and the node itself.
    coordinates: Vec<(NodeIndex, (f64, f64))>,
    width: f64,
    height: f64,
}

struct MyApp {
    scene_rect: Rect,
    gl: GraphLayout,
    graph: StableGraph<Block, BlockEdge>,
    style: NodeStyle,
}

fn main() -> eframe::Result<()> {
    let style = NodeStyle::default();

    let style_for_layout = style.clone();
    let vertex_size = move |_: NodeIndex<u32>, b: &Block| {
        let w = style_for_layout.size.x;
        let h = approx_block_height(b, &style_for_layout);
        (w as f64, h as f64)
    };

    let graph = build_dummy_cfg();

    let layout_information = rust_sugiyama::from_graph(
        &graph,
        &vertex_size,
        &Config {
            vertex_spacing: 5.0,
            ..Default::default()
        },
    )[0]
    .clone();

    let gl = GraphLayout {
        coordinates: layout_information.0,
        width: layout_information.1,
        height: layout_information.2,
    };

    eframe::run_native(
        "CFG",
        eframe::NativeOptions::default(),
        Box::new(|_cc| {
            Ok(Box::new(MyApp {
                scene_rect: Rect::from_min_size(pos2(-2000.0 * 0.5, -1000.0), vec2(2000.0, 2000.0)),
                gl,
                graph,
                style,
            }))
        }),
    )
}

fn draw_block_in_ui(ui: &mut egui::Ui, x: f32, y: f32, block: &Block, style: &NodeStyle) {
    // where the block that we're going to draw starts.
    let block_position = ui.min_rect().min + Vec2::new(x, y);

    // get the width of the content (the size of the node without the padding).
    let content_width = style.size.x - style.padding.x * 2.0;

    let body_text = block.code.join("\n");

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
    let block_height = style.header_h + style.padding.y * 2.0 + body_galley.size().y;

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
            block_rectangle.min.y + style.header_h,
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
    let label = format!("{}", block.name);
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

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            Scene::new()
                .max_inner_size([350.0, 1000.0])
                .zoom_range(0.1..=2.0)
                .show(ui, &mut self.scene_rect, |ui| {
                    for data in self.gl.coordinates.iter() {
                        let (node, (x, y)) = data;
                        let b = &self.graph[*node];
                        draw_block_in_ui(ui, *x as f32, *y as f32, b, &self.style);
                    }
                });
        });
    }
}
