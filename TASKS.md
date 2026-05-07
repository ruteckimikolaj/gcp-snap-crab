# Tasks

## Bugs / Reliability

- [ ] **Token caching** — `get_access_token()` shells out to `gcloud` on every API call; cache with TTL (`src/gcp.rs`)
- [ ] **Silent backup list failure** — returns empty `Vec` instead of propagating error; user sees empty list silently (`gcp.rs:128`)
- [ ] **Fragile tab-delimited parsing** — `gcloud` output parsed via `split('\t')`; switch to `--format=json` on all `gcloud` calls (`gcp.rs:100`)
- [ ] **Abrupt exit skips terminal cleanup** — `std::process::exit(0)` bypasses raw mode / alternate screen teardown; break loop and let `main.rs` handle it (`ui.rs:91`)
- [ ] **Hardcoded version string** — replace `"2.0.0"` with `env!("CARGO_PKG_VERSION")` (`main.rs`)

## UX / New Features

- [ ] **Scrolling in long lists** — no scroll support; lists overflow when project has many instances or backups
- [ ] **Filter / search in lists** — type to filter projects, instances, backups; standard TUI pattern, big win for large GCP environments
- [ ] **Config file / persisted defaults** — remembered projects are in-memory only; add `~/.config/gcp-snap-crab/config.toml` for default project, instance, etc.
- [ ] **Copy to clipboard** — press `y` to yank backup ID or operation ID
- [ ] **Progress indicator** — `PerformingRestore` / `PerformingCreateBackup` show static spinner; add elapsed time, poll operation for completion percentage
- [ ] **Non-interactive / scriptable mode** — `--project`, `--instance`, `--backup-id` flags to run backup/restore from CI/CD without TUI; `GcpClientTrait` already supports this
- [ ] **Export backup list** — press `e` to dump current list as JSON or CSV for auditing

## Code Quality

- [ ] **Duplicate status-check logic** — `check_restore_status` and `check_backup_status` are near-identical; extract shared `poll_operation` function (`app.rs:210-259`)
- [ ] **Duplicate render functions** — `render_source_section` and `render_target_section` share ~90% logic; parameterize into one function (`ui.rs`)
- [ ] **Missing input validation** — GCP project IDs and instance names have strict naming rules (lowercase, hyphens, max 63 chars); validate before sending to API
- [ ] **Use `--format=json` for all `gcloud` commands** — makes all parsing robust and locale-independent; prerequisite for token caching and fragile parsing fixes
