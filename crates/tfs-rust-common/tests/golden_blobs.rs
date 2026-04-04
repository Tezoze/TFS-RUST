use std::fs;
use std::path::PathBuf;
use tfs_rust_common::PropStream;

#[test]
fn test_golden_blobs() {
    let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    d.push("tests/fixtures/blobs");

    // Once actual blobs are extracted from the DB, this logic
    // will iterate over all blob files in the fixture directory
    // and verify that PropStream can parse them without error.
    if !d.exists() {
        return; // Ignore if no fixtures
    }

    if let Ok(entries) = fs::read_dir(d) {
        for entry in entries.flatten() {
            if let Ok(data) = fs::read(entry.path()) {
                if data.is_empty() {
                    continue; // skip stub files
                }
                let mut stream = PropStream::new(&data);
                // Currently just asserting we don't crash on initial read
                // (This test will be expanded once structure is mapped)
                let _ = stream.read_u8();
            }
        }
    }
}
