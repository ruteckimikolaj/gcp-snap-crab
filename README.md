![Version](https://img.shields.io/badge/version-0.2.1-blue)
![https://spdx.org/licenses/CC-BY-NC-SA-4.0.json](https://img.shields.io/badge/License-CC%20%7C%20BY--NC--SA%204.0-green)
![Platform](https://img.shields.io/badge/platform-macOS%20%7C%20Linux-blue)
![Rust Version](https://img.shields.io/badge/rust-1.70%2B-blue)
![https://crates.io/crates/gcp-snap-crab](https://img.shields.io/crates/v/gcp-snap-crab?color=blue)
![GitHub Repo Stars](https://img.shields.io/github/stars/ruteckimikolaj/gcp-snap-crab?style=social)

# 🦀 GCP Snap Crab

A minimalist, terminal-based tool to create and restore **Google Cloud SQL** database backups — no browser required.

## Table of Contents

- [✨ Features](#-features)
- [Prerequisites](#prerequisites)
- [📸 Screenshots](#-screenshots)
- [⌨️ Keyboard Shortcuts](#-keyboard-shortcuts)
- [📦 Installation](#-installation)
- [🚀 Special thanks](#-special-thanks)
- [❤️ Contributing](#-contributing)

## ✨ Features

- **Create Cloud SQL backups** — select a project, instance, and backup name through a guided TUI flow.
- **Restore Cloud SQL backups** — restore a backup from any project/instance to any target project/instance (cross-project restore supported).
- **Search & filter** — press `/` in any instance or backup list to filter by name; results update live as you type.
- **Scrollable lists** — instance and backup lists scroll with a visible scrollbar; handles large GCP environments gracefully.
- **Progress indicator** — animated spinner and elapsed time shown while a restore or backup operation is running.
- **Copy to clipboard** — press `y` to copy the current backup ID or operation ID to the system clipboard.
- **Input validation** — project IDs, instance names, and backup names are validated against GCP naming rules before submission.
- **Manual input** — press `m` to type a project ID, instance name, or backup name directly; useful for projects not listed in your active account.
- **Dry-run mode** — run with `--dry-run` to simulate the full flow without making any changes to your GCP resources.
- **Token caching** — GCP access token is cached with a TTL to avoid repeated `gcloud` calls.

## Prerequisites

- [Google Cloud SDK](https://cloud.google.com/sdk/docs/install) (`gcloud`) installed and authenticated (`gcloud auth login` / `gcloud auth application-default login`).
- A GCP project with **Cloud SQL Admin API** enabled.
- IAM permissions: `roles/cloudsql.admin` (or equivalent) on both source and target projects.

## 📸 Screenshots

![](/assets/all-gif.webp)

## ⌨️ Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `↑` / `↓` | Navigate list |
| `Enter` | Select item / confirm |
| `Esc` | Go back one step |
| `/` | Enter search/filter mode (instance & backup lists) |
| `m` | Manual input (project ID, instance name, backup name) |
| `y` | Copy highlighted backup ID or active operation ID to clipboard |
| `r` | Refresh current list or poll operation status |
| `n` | Start a new operation |
| `h` | Toggle help screen |
| `q` | Quit |

**In search mode:**

| Key | Action |
|-----|--------|
| Type | Filter the list live |
| `Enter` or `/` | Exit search, keep filter |
| `Esc` | Exit search, clear filter |

## 📦 Installation

### Using Cargo

```sh
cargo install gcp-snap-crab
```

### Using ![Homebrew](https://img.shields.io/badge/Homebrew-222222?style=for-the-badge&logo=Homebrew&logoColor=FBB040)

```shell
brew tap ruteckimikolaj/homebrew-tap
brew install gcp-snap-crab
```

### From source

```sh
git clone https://github.com/ruteckimikolaj/gcp-snap-crab
cd gcp-snap-crab
cargo build --release
./target/release/gcp-snap-crab
```

### Flags

```
gcp-snap-crab [OPTIONS]

Options:
      --dry-run    Simulate operations without making any GCP API changes
  -h, --help       Print help
  -V, --version    Print version
```

## 🚀 Special thanks

- [ratatui](https://github.com/ratatui-org/ratatui)
- [crossterm](https://github.com/crossterm-rs/crossterm)
- [tokio](https://github.com/tokio-rs/tokio)
- [reqwest](https://github.com/seanmonstar/reqwest)
- [serde](https://github.com/serde-rs/serde) / [serde_json](https://github.com/serde-rs/serde_json)
- [anyhow](https://github.com/dtolnay/anyhow)
- [chrono](https://github.com/chronotope/chrono)
- [clap](https://github.com/clap-rs/clap)
- [arboard](https://github.com/1Password/arboard)

## ❤️ Contributing

Contributions are welcome — bug reports, feature ideas, and pull requests alike.

1. Fork the repository.
2. Create a branch: `git checkout -b feature/your-feature`.
3. Make your changes and add tests where relevant.
4. Open a Pull Request describing what changed and why.
