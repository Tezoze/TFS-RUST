//! Spell / area-of-effect bitmask with transforms matching TFS `MatrixArea`.
// C++ reference: `combat.h` `MatrixArea`, `combat.cpp` flip/mirror/rotate90.

/// Row-major `rows × cols` mask; `center_x` / `center_y` are tile offsets (same convention as TFS `Center`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatrixArea {
    pub rows: u32,
    pub cols: u32,
    /// Anchor column (x) within the matrix.
    pub center_x: u32,
    /// Anchor row (y) within the matrix.
    pub center_y: u32,
    data: Vec<bool>,
}

impl MatrixArea {
    pub fn new(rows: u32, cols: u32) -> Self {
        Self {
            rows,
            cols,
            center_x: 0,
            center_y: 0,
            data: vec![false; (rows * cols) as usize],
        }
    }

    #[inline]
    pub fn get(&self, row: u32, col: u32) -> bool {
        self.data[(row * self.cols + col) as usize]
    }

    #[inline]
    pub fn set(&mut self, row: u32, col: u32, v: bool) {
        self.data[(row * self.cols + col) as usize] = v;
    }

    pub fn set_center(&mut self, row: u32, col: u32) {
        self.center_y = row;
        self.center_x = col;
    }

    /// Vertical flip (swap rows). C++ `MatrixArea::flip`.
    pub fn flip(&self) -> Self {
        let mut new_data = vec![false; self.data.len()];
        for r in 0..self.rows {
            for c in 0..self.cols {
                new_data[r as usize * self.cols as usize + c as usize] =
                    self.data[(self.rows - 1 - r) as usize * self.cols as usize + c as usize];
            }
        }
        let new_cx = self.cols - self.center_x - 1;
        let new_cy = self.center_y;
        Self {
            rows: self.rows,
            cols: self.cols,
            center_x: new_cx,
            center_y: new_cy,
            data: new_data,
        }
    }

    /// Horizontal mirror (swap columns). C++ `MatrixArea::mirror`.
    pub fn mirror(&self) -> Self {
        let mut new_data = vec![false; self.data.len()];
        for r in 0..self.rows {
            for c in 0..self.cols {
                new_data[r as usize * self.cols as usize + c as usize] =
                    self.get(r, self.cols - 1 - c);
            }
        }
        let new_cx = self.center_x;
        let new_cy = self.rows - self.center_y - 1;
        Self {
            rows: self.rows,
            cols: self.cols,
            center_x: new_cx,
            center_y: new_cy,
            data: new_data,
        }
    }

    /// Clockwise 90°; dimensions become `cols × rows`. C++ `MatrixArea::rotate90`.
    pub fn rotate90(&self) -> Self {
        let old_r = self.rows;
        let old_c = self.cols;
        let new_r = old_c;
        let new_c = old_r;
        let mut new_data = vec![false; (new_r * new_c) as usize];
        for i in 0..old_r {
            for j in 0..old_c {
                new_data[j as usize * new_c as usize + i as usize] =
                    self.data[(old_r - 1 - i) as usize * old_c as usize + j as usize];
            }
        }
        let new_cx = old_r - self.center_y - 1;
        let new_cy = self.center_x;
        Self {
            rows: new_r,
            cols: new_c,
            center_x: new_cx,
            center_y: new_cy,
            data: new_data,
        }
    }
}
