use core::f32;
use std::{
    cmp::Reverse,
    collections::{BinaryHeap, HashMap, HashSet},
};

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

    /// Add a rectangle to the cost field, with a cost radius.
    ///
    /// The cost radius isn't a hard block but discourages lines from going through it.
    pub fn add_block_rect(&mut self, block_rectangle: egui::Rect, radius: f32) {
        // we expand the rectangle by the margin so that we increase the "hitbox".
        // let block_rectangle = block_rectangle.expand(margin);

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
                let falloff = (self.grid.cell * radius - d).max(0.0) / (self.grid.cell * radius);

                *self.get_cost_cell_mut(coords) += 3.0 * falloff;
            }
        }
    }
}

#[derive(Clone)]
pub struct CellBase {
    g: f32,
    h: f32,
    parent: Option<GridCoord>,
}

impl CellBase {
    const fn new() -> Self {
        Self {
            g: f32::INFINITY,
            h: 0.0,
            parent: None,
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

    /// Used specifically so that we can have Ord on "floats".
    const fn float_key(f: f32) -> u32 {
        let bits = f.to_bits();
        if bits & 0x8000_0000 == 0 {
            bits | 0x8000_0000
        } else {
            !bits
        }
    }

    pub fn find_path(&mut self, begin: egui::Pos2, finish: egui::Pos2) -> Option<Vec<egui::Pos2>> {
        // get the starting cell.
        let start = self.field.grid.to_cell(begin);

        // get the ending cell.
        let end = self.field.grid.to_cell(finish);

        // reject if the goal is in a blocked region.
        if self.field.cost_at(end)? == f32::MAX {
            println!("invalid end position");
            return None;
        }

        // we create a bounding box that keeps our focus within range of the start and end positions.
        let bounding_box =
            egui::Rect::from_two_pos(begin, finish).expand(100.0 * self.field.grid.cell);

        // we use a min heap to keep track of the most ideal pending coordinates.
        let mut pending: BinaryHeap<(Reverse<u32>, GridCoord)> = BinaryHeap::new();

        // keep track of all the coordinates we've seen/processed.
        let mut seen: HashSet<GridCoord> = HashSet::new();

        let mut cells: HashMap<GridCoord, CellBase> = HashMap::new();

        cells.insert(
            start,
            CellBase {
                g: 0.0,
                h: Self::manhattan(start, end) as _,
                parent: None,
            },
        );

        // place the starting coordinate into the pending min heap along with its f cost.
        pending.push((Reverse(Self::float_key(cells[&start].f())), start));

        while let Some((_, mut current)) = pending.pop() {
            if !seen.insert(current) {
                continue;
            }

            if current == end {
                // this will create list of parents of successive cells.
                let mut path = vec![current];

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

            for neighbor in self.field.grid.cardinal_neighbors(current) {
                // if our neighbor doesn't exist within our assumed range then continue.
                if !bounding_box.contains(self.field.grid.cell_center(neighbor)) {
                    continue;
                }

                // take any cost as long as the cost isn't the max, aka a wall.
                let neighbor_cost = match self.field.cost_at(neighbor) {
                    Some(c) if c != f32::MAX => c,
                    _ => continue,
                };

                let incoming_dir = cells
                    .get(&current)
                    .and_then(|c| c.parent)
                    .map(|p| Grid::get_direction(p, current));

                // get the direction from our current cell to the neighbor cell, to compare.
                let step_dir = Grid::get_direction(current, neighbor);

                // if the direction is different from parent to child than child to neighbor, add penalty.
                let turn_pen = if Some(step_dir) != incoming_dir {
                    1.0
                } else {
                    0.0
                };

                // get the cost that it would take to go from our current cell to this neighbor.
                let candidate_cost = cells[&current].g + neighbor_cost + turn_pen;

                let neighbor_cell = cells.entry(neighbor).or_insert_with(CellBase::new);

                if candidate_cost < neighbor_cell.g {
                    neighbor_cell.g = candidate_cost;
                    neighbor_cell.h = Self::manhattan(neighbor, end) as _;
                    neighbor_cell.parent = Some(current);

                    let f = neighbor_cell.f();

                    pending.push((Reverse(Self::float_key(f)), neighbor));
                }
            }
        }

        None
    }
}
