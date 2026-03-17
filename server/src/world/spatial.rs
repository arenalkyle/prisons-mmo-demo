use std::collections::HashSet;

/// Spatial Hash Grid. Used so broadcasting updates are not O(N^2) and instead O(N * nearby entities)
pub(crate) struct SpatialGrid {
    cell_size: f32,
    cells: std::collections::HashMap<(i32, i32), HashSet<u64>>,
}

impl SpatialGrid {
    pub fn new(cell_size: f32) -> Self {
        Self {
            cell_size,
            cells: std::collections::HashMap::new(),
        }
    }

    fn cell_key(&self, x: f32, y: f32) -> (i32, i32) {
        (
            (x / self.cell_size).floor() as i32,
            (y / self.cell_size).floor() as i32,
        )
    }

    /// Insert a player at a position.
    pub fn insert(&mut self, id: u64, x: f32, y: f32) {
        let key = self.cell_key(x, y);
        self.cells.entry(key).or_default().insert(id);
    }

    /// Remove a player from a position.
    pub fn remove(&mut self, id: u64, x: f32, y: f32) {
        let key = self.cell_key(x, y);
        if let Some(cell) = self.cells.get_mut(&key) {
            cell.remove(&id);
            if cell.is_empty() {
                self.cells.remove(&key);
            }
        }
    }

    /// Move a player from old position to new position.
    pub fn update(&mut self, id: u64, old_x: f32, old_y: f32, new_x: f32, new_y: f32) {
        let old_key = self.cell_key(old_x, old_y);
        let new_key = self.cell_key(new_x, new_y);
        if old_key != new_key {
            self.remove(id, old_x, old_y);
            self.insert(id, new_x, new_y);
        }
    }

    /// Get all player IDs within `radius` cells of a position.
    /// For a view radius of 800px with cell_size 256, this checks ~7x7 = 49 cells max.
    pub fn query_nearby(&self, x: f32, y: f32, radius: f32) -> Vec<u64> {
        let center = self.cell_key(x, y);
        let cell_radius = (radius / self.cell_size).ceil() as i32;
        let mut result = Vec::new();

        for dx in -cell_radius..=cell_radius {
            for dy in -cell_radius..=cell_radius {
                let key = (center.0 + dx, center.1 + dy);
                if let Some(cell) = self.cells.get(&key) {
                    result.extend(cell.iter());
                }
            }
        }
        result
    }

    /// Clear all entries.
    pub fn clear(&mut self) {
        self.cells.clear();
    }
}