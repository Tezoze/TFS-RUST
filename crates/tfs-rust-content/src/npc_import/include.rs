//! Include resolution with root confinement, cycle and depth checks.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::npc_import::decode::decode_npc_bytes;
use crate::npc_import::error::{ImportError, ImportResult};
use crate::npcs::SourceSpan;

const MAX_INCLUDE_DEPTH: usize = 32;

/// Resolve `@"rel"` against `from_file`'s directory, confined to `root`.
pub fn resolve_include(
    root: &Path,
    from_file: &Path,
    relative: &str,
    span: &SourceSpan,
) -> ImportResult<PathBuf> {
    let root = root
        .canonicalize()
        .map_err(|e| ImportError::io(root, e.to_string()))?;
    let base = from_file
        .parent()
        .unwrap_or(Path::new("."))
        .canonicalize()
        .map_err(|e| ImportError::io(from_file, e.to_string()))?;
    let candidate = base.join(relative);
    let canon = candidate.canonicalize().map_err(|e| {
        ImportError::spanned(span.clone(), format!("include not found {relative:?}: {e}"))
    })?;
    if !canon.starts_with(&root) {
        return Err(ImportError::spanned(
            span.clone(),
            format!("include escapes root: {}", canon.display()),
        ));
    }
    Ok(canon)
}

pub fn read_npc_file(path: &Path) -> ImportResult<String> {
    let bytes = std::fs::read(path).map_err(|e| ImportError::io(path, e.to_string()))?;
    Ok(decode_npc_bytes(&bytes))
}

/// Track include stack for cycle/depth detection.
#[derive(Debug, Default)]
pub struct IncludeStack {
    stack: Vec<PathBuf>,
    active: HashSet<PathBuf>,
}

impl IncludeStack {
    pub fn push(&mut self, path: PathBuf, span: &SourceSpan) -> ImportResult<()> {
        if self.stack.len() >= MAX_INCLUDE_DEPTH {
            return Err(ImportError::spanned(
                span.clone(),
                format!("include depth exceeds {MAX_INCLUDE_DEPTH}"),
            ));
        }
        if !self.active.insert(path.clone()) {
            return Err(ImportError::spanned(
                span.clone(),
                format!("cyclic include: {}", path.display()),
            ));
        }
        self.stack.push(path);
        Ok(())
    }

    pub fn pop(&mut self) {
        if let Some(p) = self.stack.pop() {
            self.active.remove(&p);
        }
    }

    pub fn current(&self) -> Option<&Path> {
        self.stack.last().map(|p| p.as_path())
    }
}
