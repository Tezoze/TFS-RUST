---
trigger: always_on
---

# Rust Idioms (Mandatory)

**Not a C++ transliteration.** Reference C++ defines *what* the server must do; Rust defines *how* — idiomatically, safely, and with zero-cost patterns where possible. Same outcome, better structure. See `tfs-core.md` §Porting model.

- **Proactively use the best modern Rust methods** that achieve the exact same outcome as the original C++ code (zero-cost abstractions, iterators, pattern matching, async/await, zero-copy parsing, etc.).
- Use traits, enums with data, and pattern matching. No deep OOP hierarchies.
- Error handling: `?` operator everywhere. Top-level `anyhow`, domain errors with `thiserror`.
- Never use `.unwrap()` or `.expect()` in production code.
- Prefer zero-cost abstractions and zero-copy parsing (bytes crate, etc.).
- Performance: Use Tokio async I/O for networking and database queries. Game state mutations remain single-threaded. Only use multi-threading where it improves efficiency without changing observable behavior (e.g., parallel asset loading, not game logic).
- Strictly no `unsafe` unless user-approved.