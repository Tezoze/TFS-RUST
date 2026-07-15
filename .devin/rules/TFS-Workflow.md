---
trigger: always_on
description: >
globs:
---

# Workflow (Always Follow This Order)
1. **Plan First** — Write the porting plan to `tasks/todo.md`.
2. **C++ Analysis (772-first)** — Identify:
   - **Outcomes:** `tibia-game-master` (mechanics) and/or `gameserver` (wire)
   - **Domain shape:** which TFS surfaces the data pack needs (cylinders, conditions, Lua APIs) — keep that shape
3. **Rust Implementation** — **TFS-style domain** + **idiomatic Rust** (`TFS-rust-idioms.mdc`); era knobs in `MechanicsProfile` / formulas Lua. Preserve exact **772** observable behavior by default.
4. **Verification** — `cargo check`, `cargo clippy`, `cargo test`, logic equivalence vs 772, and that `data/` / Lua contracts still hold (1098 regression when shared paths change).
5. **Capture Lessons** — Update `tasks/lessons.md` with Rust-specific or decompile-vs-TFS insights.

**Memory & Safety**
- Strictly no `unsafe` unless user-approved for a profiled FFI or bottleneck.
- Prefer `slotmap::SlotMap` for all entity references.
- If the borrow checker is triggered, propose the cleanest idiomatic redesign that maintains identical **outcomes** and **TFS domain** contracts.

**Performance Focus**
- Always use the best Rust concurrency and zero-cost patterns available.
- Avoid unnecessary clones of heavy structs.
- Use `tokio::spawn` for I/O tasks only (network, database). Never spawn tasks that mutate game state — keep simulation logic single-threaded.
