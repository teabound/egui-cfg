use petgraph::graph::NodeIndex;

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

#[derive(Clone, Debug)]
pub struct PortLine {
    pub from: PortSlot,
    pub to: PortSlot,
}
