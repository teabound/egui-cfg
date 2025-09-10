use core::f32;
use std::{
    collections::{HashMap, HashSet},
    future::pending,
};

pub type GridCoord = (usize, usize);

#[derive(Clone, Copy, Debug)]
pub struct Grid {
    origin: egui::Pos2,
    cols: usize,
    rows: usize,
    cell: f32,
}

impl Grid {
    /// Create a grid from a rect, whose cell size is `cell`.
    pub fn from_scene(scene: egui::Rect, cell: f32) -> Self {
        // maybe return an error if either return 0 after floor.
        let cols = (scene.width() / cell).floor() as usize;
        let rows = (scene.height() / cell).floor() as usize;

        Self {
            origin: scene.min,
            cols,
            rows,
            cell,
        }
    }

    /// Convert a position to a place in the grid.
    fn to_cell(&self, p: egui::Pos2) -> GridCoord {
        // turn into origin relative coordinates.
        let rel = p - self.origin;

        // get the nearest cell.
        let x = (rel.x / self.cell).round() as usize;
        let y = (rel.y / self.cell).round() as usize;

        (x, y)
    }

    /// Convert 2D `GridCoord`, into 1D index.
    pub fn to_index(&self, coords: GridCoord) -> usize {
        coords.1 * self.cols + coords.0
    }

    /// Return the position of the center of the cell.
    pub fn cell_center(&self, coords: GridCoord) -> egui::Pos2 {
        let (x, y) = coords;
        self.origin + egui::vec2((x as f32 + 0.5) * self.cell, (y as f32 + 0.5) * self.cell)
    }
}

#[derive(Clone)]
pub struct CostField {
    cost: Vec<f32>,
    grid: Grid,
}

impl CostField {
    pub fn new(grid: Grid) -> Self {
        Self {
            cost: vec![1.0; grid.cols * grid.rows],
            grid,
        }
    }

    fn get_cost_cell_mut(&mut self, coords: GridCoord) -> &mut f32 {
        &mut self.cost[self.grid.to_index(coords)]
    }

    pub fn add_block_rect(&mut self, block_rectangle: egui::Rect, margin: f32) {
        // we expand the rectangle by the margin so that we increase the "hitbox".
        let block_rectangle = block_rectangle.expand(margin);

        for y in 0..self.grid.rows {
            for x in 0..self.grid.cols {
                let coords: GridCoord = (x, y);

                // we get the position of the center of the current grid cell.
                let cell = self.grid.cell_center(coords);

                // anything that is inside of the block increase the cost.
                if block_rectangle.contains(cell) {
                    *self.get_cost_cell_mut(coords) = f32::MAX;
                    continue;
                }

                let d = block_rectangle.distance_to_pos(cell);

                // get the 3 cell distance percentage between current cell and the block.
                let falloff = (self.grid.cell * 3.0 - d).max(0.0) / (self.grid.cell * 3.0);

                *self.get_cost_cell_mut(coords) += 3.0 * falloff;
            }
        }
    }

    fn distance_point_to_segment(p: egui::Pos2, a: egui::Pos2, b: egui::Pos2) -> f32 {
        // get the vector from a toward p.
        let ap = p - a;
        // get the vector from a toward b.
        let ab = b - a;

        // fraction of the way from A to B where P projects onto AB.
        let t = (ap.dot(ab) / ab.dot(ab)).clamp(0.0, 1.0);

        // get the vector length from q toward p.
        (a + ab * t - p).length()
    }

    /// Adds a penalty (scaled by distance) to the cost grid where polylines are placed to encourage the algorithm to
    /// not cross edges and to avoid putting the edges right next to each if possible.
    fn add_polyline_penalty(&mut self, positions: &[egui::Pos2], width: f32) {
        // return nothing if there's not a single line segment.
        if positions.len() < 2 {
            return;
        }

        for segment in positions.windows(2) {
            let a = segment[0];
            let b = segment[1];

            for y in 0..self.grid.rows {
                for x in 0..self.grid.cols {
                    let coords: GridCoord = (x, y);

                    // get the position of the center of the current coord.
                    let p = self.grid.cell_center(coords);

                    let d = Self::distance_point_to_segment(p, a, b);

                    if d <= width {
                        let t = (width - d) / width;
                        *self.get_cost_cell_mut(coords) += 5.0 * t;
                    }
                }
            }
        }
    }
}

#[derive(Clone)]
pub struct CellBase {
    g: f32,
    h: f32,
    parent: Option<GridCoord>,
    is_pending: bool,
    visited: bool,
}

impl CellBase {
    const fn new() -> Self {
        Self {
            g: f32::INFINITY,
            h: 0.0,
            parent: None,
            is_pending: false,
            visited: false,
        }
    }

    const fn f(&self) -> f32 {
        self.g + self.h
    }
}

pub struct AStar<'a> {
    field: &'a CostField,
    /// We map grid coordinates to cell information.
    cells: HashMap<GridCoord, CellBase>,
    pending: Vec<GridCoord>,
}

impl<'a> AStar<'a> {
    fn new(field: &'a CostField) -> Self {
        Self {
            field,
            cells: HashMap::new(),
            pending: Vec::new(),
        }
    }

    /// Manhattan distance that we use for our A* H cost calculation.
    const fn manhattan(a: GridCoord, b: GridCoord) -> usize {
        a.0 - b.0 + a.1 - b.1
    }

    fn pop_best(&mut self) -> GridCoord {
        let mut best_i = 0usize;
        let mut best_f = f32::INFINITY;

        // scan every cell currently waiting to be explored.
        for (i, coords) in self.pending.iter().enumerate() {
            let f = self
                .cells
                .get(coords)
                .map(|c| c.f())
                .unwrap_or(f32::INFINITY);

            // set the minimum f value as best, and its index.
            if f < best_f {
                best_f = f;
                best_i = i;
            }
        }

        // remove that best one from the pending list and return it
        self.pending.swap_remove(best_i)
    }

    pub fn find_path(&mut self, start: egui::Pos2, end: egui::Pos2) -> Option<Vec<egui::Pos2>> {
        // get the starting cell.
        let start = self.field.grid.to_cell(start);

        // get the ending cell.
        let end = self.field.grid.to_cell(end);

        self.cells = HashMap::new();

        // place the starting coordinate into the pending vector.
        self.pending = vec![start];

        let mut seen: HashSet<GridCoord> = HashSet::new();

        let start_cell = CellBase {
            g: 0.0,
            h: Self::manhattan(start, end) as _,
            parent: None,
            is_pending: true,
            visited: false,
        };

        self.cells.insert(start, start_cell);

        seen.insert(start);

        while !self.pending.is_empty() {
            // get the currently best cell.
            let current_cell = self.pop_best();

            if let Some(current_cell) = self.cells.get_mut(&current_cell) {
                current_cell.is_pending = false;

                if current_cell.visited {
                    unimplemented!()
                }

                current_cell.visited = true;
            }
        }

        None
    }
}
