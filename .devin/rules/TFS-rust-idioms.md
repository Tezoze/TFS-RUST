---
trigger: always_on
description: >
globs:
---

# Rust Idioms (Mandatory — *how* code is written)

**Not a C++ transliteration — of TFS or of the decompile.**

| Concern | Authority |
|---------|-----------|
| Observable outcomes | Decompile (772) / TFS (1098) / gameserver wire |
| Domain shape | **TFS style** — data pack, Lua APIs, cylinders, conditions |
| Implementation | **Best idiomatic Rust** |

Same decompile outcome, **TFS-shaped domain**, **better Rust structure**. Prefer a clean Rust design over matching TFS/decompile method layout, class trees, or control flow — **without** breaking the TFS data-pack / script contracts.

## Always

- **Proactively use the best modern Rust** that preserves exact outcomes (iterators, enums with data, pattern matching, traits, `?`, async/await for I/O, zero-copy parsing, etc.).
- Keep TFS-style **domain** entry points (e.g. `ConditionDamage`, cylinder moves, `Combat:execute`) so `data/` keeps working.
- Use traits and enums — **no** deep OOP hierarchies just because C++ had them.
- Error handling: `?` everywhere. Top-level `anyhow`, domain errors with `thiserror`.
- Never `.unwrap()` / `.expect()` in production code.
- Prefer zero-cost abstractions and zero-copy (`bytes`, etc.).
- Game mutations stay single-threaded; Tokio for network/DB I/O only.
- Strictly no `unsafe` unless user-approved.

## Never

- Replace TFS domain shape with decompile architecture (timer-skills, `.ndb` NPCs, etc.) — outcomes go in `MechanicsProfile`, shape stays TFS.
- Port TFS C++ *implementation* style (refcount soup, God-objects) when idiomatic Rust preserves the same domain + outcomes.
- Sacrifice clarity or safety to look like the reference `.cpp` file.

If a nicer Rust approach would change any observable result **or** break the data-pack/Lua contract, document and ask before diverging.
