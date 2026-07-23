//! Import errors with source spans.

use std::path::PathBuf;

use thiserror::Error;

use crate::npcs::SourceSpan;

/// Offline NPC importer error.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ImportError {
    #[error("{}: {message}", span.display())]
    Spanned { span: SourceSpan, message: String },
    #[error("{path}: {message}")]
    Io { path: PathBuf, message: String },
    #[error("{message}")]
    Message { message: String },
}

impl ImportError {
    pub fn spanned(span: SourceSpan, message: impl Into<String>) -> Self {
        Self::Spanned {
            span,
            message: message.into(),
        }
    }

    pub fn io(path: impl Into<PathBuf>, message: impl Into<String>) -> Self {
        Self::Io {
            path: path.into(),
            message: message.into(),
        }
    }

    pub fn msg(message: impl Into<String>) -> Self {
        Self::Message {
            message: message.into(),
        }
    }
}

pub type ImportResult<T> = Result<T, ImportError>;
