# Project: core

## MCP tool usage (mandatory)

This repo ships four MCP servers in `.mcp.json`. All `✓ Connected`. Tools are **deferred** — load schemas via `ToolSearch` before first call, then invoke. Do **not** fall back to plain `grep` / `find` / guesswork when an MCP fits the task.

Decision order for code questions:
1. **Symbol-level intent** ("find references", "rename", "what calls X") → `precision-search` (Serena LSP).
2. **Structural / tree shape** ("show all classes in file", "method signatures", AST queries) → `structural-explorer`.
3. **Free-text / fuzzy lookup** across repo → `codebase-index`.
4. **External library API / docs** → `context7`.

### 1. `precision-search` (Serena, LSP-backed)

Best tool in this repo for semantic code work. Uses LSP — knows definitions, references, types.

Load: `ToolSearch query="precision-search serena symbol"` (or `select:` once names known).

Use for:
- find symbol definition / all references / call sites
- rename symbol, safe edits via symbol body replace
- list symbols in file/module
- onboarding a new area: `get_symbols_overview` first, then drill in
- writing/reading project memories Serena maintains in `.serena/memories/`

Skip for: pure text search on non-code files (use `codebase-index`).

### 2. `structural-explorer` (tree-sitter-analyzer)

AST-level inspection. Faster than Serena for shape questions; no LSP startup.

Load: `ToolSearch query="structural-explorer tree-sitter analyze"`.

Use for:
- "list all functions/classes in file X"
- method signatures, line ranges, decorators
- partial reads of huge files by AST node (avoid loading 5k-line files via `Read`)
- generating code-graph snapshots into `.codegraph/`

Skip for: cross-file reference search (use Serena).

### 3. `codebase-index` (`mcp__codebase-index__*`)

Indexed text/code search. Fastest for "where does string X appear".

Load: `ToolSearch query="select:mcp__codebase-index__search_code_advanced,mcp__codebase-index__find_files,mcp__codebase-index__get_file_summary,mcp__codebase-index__get_symbol_body"`.

Use for:
- `search_code_advanced` — keyword / regex across repo
- `find_files` — glob (replaces bash `find`)
- `get_file_summary` — quick orientation on a large module
- `refresh_index` after large refactors / pulls
- First clone: `set_project_path` → `build_deep_index`

### 4. `context7` (`mcp__context7__*`)

Up-to-date library docs. Training data is stale.

Load: `ToolSearch query="select:mcp__context7__resolve-library-id,mcp__context7__query-docs"`.

Trigger on **any** mention of a library / framework / SDK / CLI: Django, SQLAlchemy, pydantic, FastAPI, google-cloud-pubsub, pytest, uv, etc. Flow: `resolve-library-id` → `query-docs`.

Skip for: business-logic debugging, internal refactors, general programming concepts.

**Note:** `plugin:context7:context7` (npx) is duplicate of project `context7` (HTTP). Prefer the HTTP one.
