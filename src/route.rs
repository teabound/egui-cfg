use core::f32;

pub type GridCoord = (usize, usize);

#[derive(Clone, Copy, Debug)]
struct Grid {
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
struct CostField {
    cost: Vec<f32>,
    grid: Grid,
}

impl CostField {
    fn new(grid: Grid) -> Self {
        Self {
            cost: vec![1.0; grid.cols * grid.rows],
            grid,
        }
    }

    fn get_cost_cell_mut(&mut self, coords: GridCoord) -> &mut f32 {
        &mut self.cost[self.grid.to_index(coords)]
    }

    fn add_block_rect(&mut self, block_rectangle: egui::Rect, margin: f32) {
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
