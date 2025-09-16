use core::f32;
use std::collections::{HashMap, HashSet};

pub type GridCoord = (usize, usize);

#[derive(Clone, Copy, Debug)]
pub struct Grid {
    pub origin: egui::Pos2,
    pub cols: usize,
    pub rows: usize,
    pub cell: f32,
}

impl Grid {
    /// Create a grid from a rect, whose cell size is `cell`.
    pub fn from_scene(scene: egui::Rect, cell: f32) -> Self {
        // maybe return an error if either return 0 after floor.
        let cols = (scene.width() / cell).floor() as usize;
        let rows = (scene.height() / cell).floor() as usize;

        let ox = (scene.min.x / cell).floor() * cell;
        let oy = (scene.min.y / cell).floor() * cell;

        Self {
            origin: egui::pos2(ox, oy),
            cols,
            rows,
            cell,
        }
    }

    /// Gets all valid 4-direction neighbors of `coords` inside the grid.
    fn cardinal_neighbors(&self, coords: GridCoord) -> Vec<GridCoord> {
        let (x, y) = coords;
        let mut neighbors = Vec::new();

        if x + 1 < self.cols {
            neighbors.push((x + 1, y));
        }
        if let Some(nx) = x.checked_sub(1) {
            neighbors.push((nx, y));
        }

        if y + 1 < self.rows {
            neighbors.push((x, y + 1));
        }

        if let Some(ny) = y.checked_sub(1) {
            neighbors.push((x, ny));
        }

        neighbors
    }

    /// Convert a position to a place in the grid.
    fn to_cell(&self, p: egui::Pos2) -> GridCoord {
        // turn into origin relative coordinates.
        let rel = p - self.origin;

        // get the nearest cell.
        let mut x = (rel.x / self.cell).floor() as isize;
        let mut y = (rel.y / self.cell).floor() as isize;

        x = x.clamp(0, self.cols as isize - 1);
        y = y.clamp(0, self.rows as isize - 1);

        (x as usize, y as usize)
    }

    /// Convert 2D `GridCoord`, into 1D index.
    pub const fn to_index(&self, coords: GridCoord) -> usize {
        coords.1 * self.cols + coords.0
    }

    /// Return the position of the center of the cell.
    pub fn cell_center(&self, coords: GridCoord) -> egui::Pos2 {
        let (x, y) = coords;
        self.origin + egui::vec2((x as f32 + 0.5) * self.cell, (y as f32 + 0.5) * self.cell)
    }

    /// Returns which direction we go to, from `a`, to `b`.
    pub const fn get_direction(a: GridCoord, b: GridCoord) -> (i8, i8) {
        (
            (b.0 as isize - a.0 as isize).signum() as i8,
            (b.1 as isize - a.1 as isize).signum() as i8,
        )
    }
}

#[derive(Debug, Clone)]
pub struct CostField {
    pub cost: Vec<f32>,
    pub grid: Grid,
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

    fn cost_at(&self, coords: GridCoord) -> Option<f32> {
        self.cost.get(self.grid.to_index(coords)).copied()
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
    pub fn add_polyline_penalty(&mut self, positions: &[egui::Pos2], width: f32) {
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
}

impl CellBase {
    const fn new() -> Self {
        Self {
            g: f32::INFINITY,
            h: 0.0,
            parent: None,
            is_pending: false,
        }
    }

    const fn f(&self) -> f32 {
        self.g + self.h
    }
}

pub struct AStar<'a> {
    field: &'a CostField,
    // /// We map grid coordinates to cell information.
}

impl<'a> AStar<'a> {
    pub fn new(field: &'a CostField) -> Self {
        Self { field }
    }

    /// Manhattan distance that we use for our A* H cost calculation.
    const fn manhattan(a: GridCoord, b: GridCoord) -> usize {
        a.0.abs_diff(b.0) + a.1.abs_diff(b.1)
    }

    fn pop_best(
        &mut self,
        pending: &mut Vec<GridCoord>,
        cells: &HashMap<GridCoord, CellBase>,
    ) -> GridCoord {
        let mut best_i = 0usize;
        let mut best_f = f32::INFINITY;

        // scan every cell currently waiting to be explored.
        for (i, coords) in pending.iter().enumerate() {
            let f = cells.get(coords).map(|c| c.f()).unwrap_or(f32::INFINITY);

            // set the minimum f value as best, and its index.
            if f < best_f {
                best_f = f;
                best_i = i;
            }
        }

        // remove that best one from the pending list and return it
        pending.swap_remove(best_i)
    }

    pub fn find_path(&mut self, start: egui::Pos2, end: egui::Pos2) -> Option<Vec<egui::Pos2>> {
        // get the starting cell.
        let start = self.field.grid.to_cell(start);

        // get the ending cell.
        let end = self.field.grid.to_cell(end);

        // reject if the goal is in a blocked region.
        if self.field.cost_at(end)? == f32::MAX {
            println!("invalid end position");
            return None;
        }

        let mut cells: HashMap<GridCoord, CellBase> = HashMap::new();

        // place the starting coordinate into the pending vector.
        let mut pending: Vec<GridCoord> = vec![start];

        let mut seen: HashSet<GridCoord> = HashSet::new();

        let start_cell = CellBase {
            g: 0.0,
            h: Self::manhattan(start, end) as _,
            parent: None,
            is_pending: true,
        };

        cells.insert(start, start_cell);

        seen.insert(start);

        while !pending.is_empty() {
            // get the currently best cell.
            let current_cell = self.pop_best(&mut pending, &cells);

            if let Some(current_cell) = cells.get_mut(&current_cell) {
                current_cell.is_pending = false;
            }

            if current_cell == end {
                // this will create list of parents of successive cells.
                let mut path = vec![current_cell];

                let mut current = current_cell;

                while let Some(prev) = cells.get(&current).and_then(|c| c.parent) {
                    current = prev;
                    path.push(current);
                }

                // reverse the list so that it's children->parent.
                path.reverse();

                return Some(
                    path.into_iter()
                        .map(|p| self.field.grid.cell_center(p))
                        .collect(),
                );
            }

            for neighbor in self.field.grid.cardinal_neighbors(current_cell) {
                let neighbor_cost = match self.field.cost_at(neighbor) {
                    Some(c) => c,
                    None => continue,
                };

                if neighbor_cost == f32::MAX {
                    continue;
                }

                if !seen.contains(&neighbor) {
                    cells.insert(neighbor, CellBase::new());
                    seen.insert(neighbor);
                }

                let incoming_dir = cells
                    .get(&current_cell)
                    .and_then(|c| c.parent)
                    .map(|p| Grid::get_direction(p, current_cell));

                // get the direction from our current cell to the neighbor cell, to compare.
                let step_dir = Grid::get_direction(current_cell, neighbor);

                // if the direction is different from parent to child than child to neighbor, add penalty.
                let turn_pen = if Some(step_dir) != incoming_dir {
                    1.0
                } else {
                    0.0
                };

                // get the cost that it would take to go from our current cell to this neighbor.
                let candidate_cost = cells[&current_cell].g + neighbor_cost + turn_pen;

                if candidate_cost < cells[&neighbor].g {
                    let neighbor_cell = cells.get_mut(&neighbor).unwrap();

                    neighbor_cell.parent = Some(current_cell);
                    neighbor_cell.g = candidate_cost;
                    neighbor_cell.h = AStar::manhattan(neighbor, end) as _;

                    if !neighbor_cell.is_pending {
                        pending.push(neighbor);
                        neighbor_cell.is_pending = true;
                    }
                }
            }
        }

        None
    }
}
