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

    // /// Convert 2D `GridCoord`, into 1D index.
    // fn to_index(&self, coords: GridCoord) -> usize {
    //     coords.1 * self.cols + coords.0
    // }

    /// Return the position of the center of the cell.
    fn cell_center(&self, coords: GridCoord) -> egui::Pos2 {
        let (x, y) = coords;
        self.origin + egui::vec2((x as f32 + 0.5) * self.cell, (y as f32 + 0.5) * self.cell)
    }
}

#[derive(Clone)]
struct Field {
    cost: Vec<f32>,
    grid: Grid,
}
