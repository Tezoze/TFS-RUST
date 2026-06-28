---
inclusion: auto
name: tfs-code-hygiene
description: Code hygiene — deduplication, naming, dead-code removal, and reuse of existing helpers.
---

# Code Hygiene (Mandatory)

Keep ports readable and maintainable as cylinder arms and move paths grow.

## Extract Before Duplication Multiplies

When two or more paths share structure (room check → count transfer → broadcast), extract a **small core helper** and keep per-arm side effects (tile vs slot vs container UI) at the call site.

```rust
// ❌ BAD — same room/count/notify block copied in Tile→Inv and Inv→Container arms
// ✅ GOOD — merge_partial_stack_counts + merge_detached_stack_counts; broadcasts stay local
```

Extract **before** adding the next cylinder arm, not after the third copy.

## Name Helpers for Their Contract

Helper names must spell out **preconditions**, not just what they do:

| Pattern | Name implies |
|---------|----------------|
| Source still on cylinder | `merge_partial_stack_counts` — subtract source + add target |
| Source already detached | `merge_detached_stack_counts` — add target only |

Document in doc comments **when to use each** and cross-link the paired helper. Add a one-line inline comment at non-obvious call sites (e.g. partial vs full merge in the same `if` block).

## Reuse Existing Scoped Helpers

Before adding `std::mem::take(&mut self.container_registry)` (or similar borrow-splitting), check for an existing wrapper — e.g. `hydrate_container_if_needed` already scopes registry access + chain refresh.

If the pattern appears 3+ times and no wrapper exists, add one scoped helper instead of copying the block.

## Remove Dead Code in the Same PR

After refactors that add early returns or outer `if let` guards, **delete unreachable inner blocks** in the same change — do not leave duplicate paths "just in case."

## Comment Non-Obvious Indirection

When a function maps between representations (slot byte → `queryMaxCount` index, flags → C++ index), add a one-line doc comment with the C++ reference. Skip comments on self-explanatory code.

## Scope Discipline

- Minimal diff: fix the task, do not drive-by refactor unrelated code.
- No one-off helpers for a single call site unless duplication is imminent.
- Prefer deleting noise over adding abstraction for abstraction's sake.
