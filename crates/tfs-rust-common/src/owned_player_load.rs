//! Opaque owned player DB load for game-thread apply.
//!
//! Keeps `tfs-rust-common` free of `tfs-rust-db` / sqlx while still allowing
//! `GameCommand::PlayerLoaded` to carry owned load data across the I/O → game
//! thread boundary. Core boxes `LoadedPlayerData`; the game loop downcasts.

use std::any::Any;
use std::fmt;

/// Type-erased `LoadedPlayerData` (or test doubles) for [`crate::GameCommand::PlayerLoaded`].
pub struct OwnedPlayerLoad {
    inner: Box<dyn Any + Send>,
}

impl OwnedPlayerLoad {
    pub fn new<T: Send + 'static>(value: T) -> Self {
        Self {
            inner: Box::new(value),
        }
    }

    pub fn downcast<T: Send + 'static>(self) -> Result<T, Self> {
        match self.inner.downcast::<T>() {
            Ok(boxed) => Ok(*boxed),
            Err(inner) => Err(Self { inner }),
        }
    }
}

impl fmt::Debug for OwnedPlayerLoad {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("OwnedPlayerLoad(..)")
    }
}
