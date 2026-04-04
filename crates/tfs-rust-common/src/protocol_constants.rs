//! Protocol sizes and viewport constants (game world / map).
// C++ reference (this repo): `src/map.h`, `src/const.h`.

/// `Map::MAP_MAX_LAYERS` — used by `GetMapDescription` z-loop.
pub const MAP_MAX_LAYERS: i32 = 16;

/// `Map::maxClientViewportX`
pub const MAX_CLIENT_VIEWPORT_X: i32 = 8;

/// `Map::maxClientViewportY`
pub const MAX_CLIENT_VIEWPORT_Y: i32 = 6;

/// Width/height passed to `GetMapDescription` for full screen: `(maxClientViewportX * 2) + 2`, etc.
#[inline]
pub fn client_viewport_width() -> i32 {
    (MAX_CLIENT_VIEWPORT_X * 2) + 2
}

#[inline]
pub fn client_viewport_height() -> i32 {
    (MAX_CLIENT_VIEWPORT_Y * 2) + 2
}
