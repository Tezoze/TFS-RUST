//! Entity identifiers backed by generational slot maps.
// C++ reference: `Creature*` / `Item*` identity — Rust uses stable keys instead.

slotmap::new_key_type! {
    /// Stable creature reference (`Creature` in TFS).
    pub struct CreatureId;
    /// Stable item reference (`Item` in TFS).
    pub struct ItemId;
}
