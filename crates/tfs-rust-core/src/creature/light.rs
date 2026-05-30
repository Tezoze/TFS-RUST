//! Creature / item light — `LightInfo`, `Player::itemsLight` (`src/player.h`, `src/item.h`).
// C++ reference: `Item::getLightInfo` (`item.cpp`), `Player::updateItemsLight` (`player.cpp`).

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LightInfo {
    pub level: u8,
    pub color: u8,
}

impl LightInfo {
    /// C++ `Player::getCreatureLight` — pick brighter of two sources.
    #[inline]
    pub fn max_of(a: Self, b: Self) -> Self {
        if a.level >= b.level {
            a
        } else {
            b
        }
    }
}
