//! Quadtree over (x, y) with 32×32 leaves and per-leaf spectator cache invalidation.
// C++ reference: `map.cpp` `QTreeNode`, spectator caching.
//
//! After invalidation (creature enter/leave), the first query that hits a leaf copies
//! `creatures` into `cached_spectators`; later queries in that leaf use the cache slice only.

use crate::creature::creature_id_eq_slice;
use crate::ids::CreatureId;
use tfs_rust_common::Position;

/// One node in the spatial index. Covers axis-aligned rectangle [x0,x1]×[y0,y1] inclusive.
#[derive(Debug)]
pub enum QTreeNode {
    Branch {
        nw: Box<QTreeNode>,
        ne: Box<QTreeNode>,
        sw: Box<QTreeNode>,
        se: Box<QTreeNode>,
        x0: u16,
        y0: u16,
        x1: u16,
        y1: u16,
    },
    Leaf {
        x0: u16,
        y0: u16,
        x1: u16,
        y1: u16,
        creatures: Vec<CreatureId>,
        cached_spectators: Option<Vec<CreatureId>>,
    },
}

impl QTreeNode {
    pub fn build(x0: u16, y0: u16, x1: u16, y1: u16) -> Self {
        let w = x1.saturating_sub(x0).saturating_add(1);
        let h = y1.saturating_sub(y0).saturating_add(1);
        if w <= 32 && h <= 32 {
            return QTreeNode::Leaf {
                x0,
                y0,
                x1,
                y1,
                creatures: Vec::new(),
                cached_spectators: None,
            };
        }

        let mx = x0.saturating_add(w / 2).min(x1);
        let my = y0.saturating_add(h / 2).min(y1);

        let nw = Self::build(x0, y0, mx, my);
        let ne = if mx < x1 {
            Self::build(mx + 1, y0, x1, my)
        } else {
            Self::empty_leaf(x0, y0, x0, y0)
        };
        let sw = if my < y1 {
            Self::build(x0, my + 1, mx, y1)
        } else {
            Self::empty_leaf(x0, y0, x0, y0)
        };
        let se = if mx < x1 && my < y1 {
            Self::build(mx + 1, my + 1, x1, y1)
        } else {
            Self::empty_leaf(x0, y0, x0, y0)
        };

        QTreeNode::Branch {
            nw: Box::new(nw),
            ne: Box::new(ne),
            sw: Box::new(sw),
            se: Box::new(se),
            x0,
            y0,
            x1,
            y1,
        }
    }

    fn empty_leaf(x0: u16, y0: u16, x1: u16, y1: u16) -> Self {
        QTreeNode::Leaf {
            x0,
            y0,
            x1,
            y1,
            creatures: Vec::new(),
            cached_spectators: None,
        }
    }

    pub fn insert_creature(&mut self, pos: Position, id: CreatureId) {
        Self::insert_inner(self, pos.x, pos.y, id);
    }

    fn insert_inner(node: &mut QTreeNode, x: u16, y: u16, id: CreatureId) {
        match node {
            QTreeNode::Branch {
                nw,
                ne,
                sw,
                se,
                x0,
                y0,
                x1,
                y1,
                ..
            } => {
                let mx = mid(*x0, *x1);
                let my = mid(*y0, *y1);
                let target = if x <= mx {
                    if y <= my {
                        nw.as_mut()
                    } else {
                        sw.as_mut()
                    }
                } else if y <= my {
                    ne.as_mut()
                } else {
                    se.as_mut()
                };
                Self::insert_inner(target, x, y, id);
            }
            QTreeNode::Leaf {
                creatures,
                cached_spectators,
                x0,
                y0,
                x1,
                y1,
            } => {
                if !(x >= *x0 && x <= *x1 && y >= *y0 && y <= *y1) {
                    return;
                }
                creatures.push(id);
                *cached_spectators = None;
            }
        }
    }

    pub fn remove_creature(&mut self, pos: Position, id: CreatureId) -> bool {
        Self::remove_inner(self, pos.x, pos.y, id)
    }

    fn remove_inner(node: &mut QTreeNode, x: u16, y: u16, id: CreatureId) -> bool {
        match node {
            QTreeNode::Branch {
                nw,
                ne,
                sw,
                se,
                x0,
                y0,
                x1,
                y1,
                ..
            } => {
                let mx = mid(*x0, *x1);
                let my = mid(*y0, *y1);
                let target = if x <= mx {
                    if y <= my {
                        nw.as_mut()
                    } else {
                        sw.as_mut()
                    }
                } else if y <= my {
                    ne.as_mut()
                } else {
                    se.as_mut()
                };
                Self::remove_inner(target, x, y, id)
            }
            QTreeNode::Leaf {
                creatures,
                cached_spectators,
                ..
            } => {
                if let Some(i) = creature_id_eq_slice(creatures, id) {
                    creatures.swap_remove(i);
                    *cached_spectators = None;
                    return true;
                }
                false
            }
        }
    }

    /// Collect spectators within Chebyshev distance `range` of `center` (same floor).
    pub fn get_spectators(&mut self, center: Position, range: u16) -> Vec<CreatureId> {
        let r = range as i32;
        let min_x = (center.x as i32 - r).max(0) as u16;
        let max_x = (center.x as i32 + r).min(u16::MAX as i32) as u16;
        let min_y = (center.y as i32 - r).max(0) as u16;
        let max_y = (center.y as i32 + r).min(u16::MAX as i32) as u16;
        let mut out = Vec::new();
        Self::collect_rect(self, min_x, min_y, max_x, max_y, &mut out);
        out.sort();
        out.dedup();
        out
    }

    fn collect_rect(
        node: &mut QTreeNode,
        rx0: u16,
        ry0: u16,
        rx1: u16,
        ry1: u16,
        out: &mut Vec<CreatureId>,
    ) {
        match node {
            QTreeNode::Branch {
                nw,
                ne,
                sw,
                se,
                x0,
                y0,
                x1,
                y1,
                ..
            } => {
                if rects_overlap(*x0, *y0, *x1, *y1, rx0, ry0, rx1, ry1) {
                    Self::collect_rect(nw.as_mut(), rx0, ry0, rx1, ry1, out);
                    Self::collect_rect(ne.as_mut(), rx0, ry0, rx1, ry1, out);
                    Self::collect_rect(sw.as_mut(), rx0, ry0, rx1, ry1, out);
                    Self::collect_rect(se.as_mut(), rx0, ry0, rx1, ry1, out);
                }
            }
            QTreeNode::Leaf {
                x0,
                y0,
                x1,
                y1,
                creatures,
                cached_spectators,
            } => {
                if !rects_overlap(*x0, *y0, *x1, *y1, rx0, ry0, rx1, ry1) {
                    return;
                }
                match cached_spectators {
                    Some(ref cache) => out.extend_from_slice(cache),
                    None => {
                        out.extend_from_slice(creatures);
                        *cached_spectators = Some(creatures.clone());
                    }
                }
            }
        }
    }
}

fn mid(a: u16, b: u16) -> u16 {
    a.saturating_add(b.saturating_sub(a) / 2)
}

#[allow(clippy::too_many_arguments)]
fn rects_overlap(
    ax0: u16,
    ay0: u16,
    ax1: u16,
    ay1: u16,
    bx0: u16,
    by0: u16,
    bx1: u16,
    by1: u16,
) -> bool {
    ax0 <= bx1 && bx0 <= ax1 && ay0 <= by1 && by0 <= ay1
}

#[cfg(test)]
mod tests {
    use super::*;
    use slotmap::SlotMap;

    #[test]
    fn spectator_queries_stable_and_cache_populated() {
        let mut tree = QTreeNode::build(0, 0, 31, 31);
        let mut sm: SlotMap<CreatureId, ()> = SlotMap::with_key();
        let id = sm.insert(());
        tree.insert_creature(Position::new(10, 10, 0), id);

        let center = Position::new(10, 10, 0);
        let first = tree.get_spectators(center, 5);
        let second = tree.get_spectators(center, 5);
        assert_eq!(first, second);
        assert_eq!(first, vec![id]);
    }
}
