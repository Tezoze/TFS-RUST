<!-- code-review-graph MCP tools -->

## Cursor rules (binding for every task)

Treat `AGENTS.md` and all `.mdc` files under `.cursor/rules/` as project rules. If they are not in context, add them with `@` (e.g. `@.cursor/rules/TFS-Core.mdc`).

**Always apply** (see `alwaysApply: true` in each file): `TFS-Core.mdc`, `TFS-Workflow.mdc`, `TFS-cpp-references.mdc`, `TFS-entity-storage.mdc`, `TFS-rust-idioms.mdc`, `TFS-threading.mdc`, `tfs-output-format.mdc` (output format; applies broadly via `globs`).

**Path-scoped** (auto-attach when editing matching files; `@`-mention when the task touches those areas without those files open): `TFS-packets.mdc` (net), `TFS-database.mdc` (db), `TFS-lua-boundaries.mdc` (core + lua).

## MCP Tools: code-review-graph

**IMPORTANT: This project has a knowledge graph. ALWAYS use the
code-review-graph MCP tools BEFORE using Grep/Glob/Read to explore
the codebase.** The graph is faster, cheaper (fewer tokens), and gives
you structural context (callers, dependents, test coverage) that file
scanning cannot.

### When to use graph tools FIRST

- **Exploring code**: `semantic_search_nodes` or `query_graph` instead of Grep
- **Understanding impact**: `get_impact_radius` instead of manually tracing imports
- **Code review**: `detect_changes` + `get_review_context` instead of reading entire files
- **Finding relationships**: `query_graph` with callers_of/callees_of/imports_of/tests_for
- **Architecture questions**: `get_architecture_overview` + `list_communities`

Fall back to Grep/Glob/Read **only** when the graph doesn't cover what you need.

### Key Tools

| Tool | Use when |
|------|----------|
| `detect_changes` | Reviewing code changes — gives risk-scored analysis |
| `get_review_context` | Need source snippets for review — token-efficient |
| `get_impact_radius` | Understanding blast radius of a change |
| `get_affected_flows` | Finding which execution paths are impacted |
| `query_graph` | Tracing callers, callees, imports, tests, dependencies |
| `semantic_search_nodes` | Finding functions/classes by name or keyword |
| `get_architecture_overview` | Understanding high-level codebase structure |
| `refactor_tool` | Planning renames, finding dead code |

### Workflow

1. The graph auto-updates on file changes (via hooks).
2. Use `detect_changes` for code review.
3. Use `get_affected_flows` to understand impact.
4. Use `query_graph` pattern="tests_for" to check coverage.
