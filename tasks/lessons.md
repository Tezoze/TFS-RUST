# Engine Architecture & Implementation Lessons

1. **Legacy C++ Parity**: Always refer back to the exact legacy TFS C++ source code when answering questions, reproducing behaviors, or making architectural decisions for edge cases. Do not assume behavior; find the correct `.cpp` file using `grep_search` in the legacy `src/` directory and mirror its logic. 
*(Added: April 2026 after spawning parser error logic mismatch)*
