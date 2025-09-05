use thiserror::Error;

// #[derive(Error, Debug)]
// pub enum GraphLayoutError {
//     #[error("Could not find entry point node in graph.")]
//     NoEntryPoint,
//     #[error("Could not find edge in graph.")]
//     NoEdgeFound,
// }

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
