---
trigger: always_on
description: Exact response format and style rules for all interactions in the TFS project.
globs: ["**/*"]
---

# Output Format (Always Use This Structure)
- **Step 1:** Summary of plan + affected files + any better Rust patterns being used.
- **Step 2:** Code changes in unified diff format (or full file if small). Highlight where a superior Rust method replaces the C++ logic while preserving exact behavior.
- **Step 3:** Suggested `cargo` commands for verification.
- **Step 4:** Any tests that should be added/updated.
- Be concise. Only add explanations when explicitly asked. No fluff.

# Core Principles
- **Decompile outcomes** (772) + **TFS-style domain** (data pack / Lua) + **idiomatic Rust** implementation.
- Do not replace TFS domain shape with decompile architecture; do not transliterate TFS C++ style into Rust.
- Observable parity is paramount (not line-for-line C++).
- Translate Tibia domain knowledge into superior Rust structures while preserving exact active-era behavior and data-pack contracts.