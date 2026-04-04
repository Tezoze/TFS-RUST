//! World `Item` node (inventory / map stack modeled in later phases).
// C++ reference: `Item` (`item.h`).

#[derive(Debug, Clone)]
pub struct Item {
    pub item_type: u16,
    pub count: u16,
}
