//! Stable TCP connection id (game protocol routing).
// C++ reference: implicit `Connection*` / player session.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ConnId(pub u32);
