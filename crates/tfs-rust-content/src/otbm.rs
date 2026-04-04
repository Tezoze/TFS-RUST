use std::path::Path;
use tfs_rust_common::error::{Result, TfsRustError};
use tracing::info;

pub struct MapData {
    // Defines tiles, towns, waypoints
}

pub struct OtbmLoader;

impl OtbmLoader {
    pub fn load_from_file(path: &Path) -> Result<MapData> {
        info!("Loading OTBM map from {:?}", path);
        let _data = std::fs::read(path).map_err(|e| TfsRustError::Content {
            file: path.to_string_lossy().into(),
            message: e.to_string(),
        })?;

        // TODO: Detailed tree block parsing of OTBM spec.
        Ok(MapData {})
    }
}
