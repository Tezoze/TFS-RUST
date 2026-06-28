---
inclusion: auto
name: tfs-output-format
description: Exact response format and style rules for all interactions in the TFS project.
---

# Output Format (Always Use This Structure)
- **Step 1:** Summary of plan + affected files + any better Rust patterns being used.
- **Step 2:** Code changes in unified diff format (or full file if small). Highlight where a superior Rust method replaces the C++ logic while preserving exact behavior.
- **Step 3:** Suggested `cargo` commands for verification.
- **Step 4:** Any tests that should be added/updated.
- Be concise. Only add explanations when explicitly asked. No fluff.

# Core Principles
- Safety, performance, and idiomatic Rust over direct C++ translation — but only when the outcome is *identical*.
- Compatibility and 1:1 parity are paramount.
- You are translating Tibia domain knowledge into superior Rust structures while preserving exact behavior.
