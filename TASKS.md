# Tasks

## Bugs / Reliability

- [x] **Token caching** — `get_access_token()` shells out to `gcloud` on every API call; cache with TTL (`src/gcp.rs`)
- [x] **Silent backup list failure** — returns empty `Vec` instead of propagating error; user sees empty list silently (`gcp.rs:128`)
- [x] **Fragile tab-delimited parsing** — `gcloud` output parsed via `split('\t')`; switch to `--format=json` on all `gcloud` calls (`gcp.rs:100`)
- [x] **Abrupt exit skips terminal cleanup** — `std::process::exit(0)` bypasses raw mode / alternate screen teardown; break loop and let `main.rs` handle it (`ui.rs:91`)
- [x] **Hardcoded version string** — replace `"2.0.0"` with `env!("CARGO_PKG_VERSION")` (`main.rs`)

## UX / New Features

- [x] **Scrolling in long lists** — no scroll support; lists overflow when project has many instances or backups
- [x] **Filter / search in lists** (redesigned) — type to filter projects, instances, backups; standard TUI pattern, big win for large GCP environments
- [ ] **Config file / persisted defaults** — remembered projects are in-memory only; add `~/.config/gcp-snap-crab/config.toml` for default project, instance, etc.
- [ ] **Copy to clipboard** — press `y` to yank backup ID or operation ID
- [ ] **Progress indicator** — `PerformingRestore` / `PerformingCreateBackup` show static spinner; add elapsed time, poll operation for completion percentage
- [ ] **Non-interactive / scriptable mode** — `--project`, `--instance`, `--backup-id` flags to run backup/restore from CI/CD without TUI; `GcpClientTrait` already supports this
- [ ] **Export backup list** — press `e` to dump current list as JSON or CSV for auditing

## Code Quality

- [x] **Duplicate status-check logic** — extracted `poll_operation` shared helper in `app.rs`
- [x] **Duplicate render functions** — extracted `render_step_box` helper; eliminated ~200 lines of repeated project/instance/backup box rendering across source, target, and backup flows
- [x] **Missing input validation** — `src/validation.rs` added; project IDs (6–30 chars, lowercase, letter-start, no trailing hyphen), instance names (1–98 chars), backup names (1–63 chars, alphanumeric/hyphen/underscore) validated in `finish_manual_input`
- [x] **Use `--format=json` for all `gcloud` commands** — makes all parsing robust and locale-independent; done for all list commands
- [x] **Split `ui.rs` into modules** — split into `ui/mod.rs` (run_app + input handlers), `ui/render.rs` (section renderers), `ui/popups.rs` (popup renderers), `ui/widgets.rs` (reusable leaf widgets)
- [ ] **Split `app.rs` into modules** — 593 lines; extract flow logic into `app/restore.rs` and `app/backup.rs`, keep `app/mod.rs` as thin coordinator
