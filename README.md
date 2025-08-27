![Version](https://img.shields.io/badge/version-0.1.0-blue)
![https://spdx.org/licenses/CC-BY-NC-SA-4.0.json](https://img.shields.io/badge/License-CC%20%7C%20BY--NC--SA%204.0-green)
![Platform](https://img.shields.io/badge/platform-macOS%20%7C%20Linux%20%7C%20Windows-blue)
![Rust Version](https://img.shields.io/badge/rust-1.70.0-blue)
![https://crates.io/crates/gcp-snap-crab](https://img.shields.io/crates/v/gcp-snap-crab?color=blue)
![Homebrew](https://img.shields.io/badge/homebrew-coming%20soon-orange)
![GitHub Repo Stars](https://img.shields.io/github/stars/ruteckimikolaj/gcp-snap-crab?style=social)

# 🦀 GCP Snap Crab

A minimalist set of scripts to restore Google Cloud Platform (GCP) resources from backups.

## Table of Contents

- [✨ Features](#-features)
- [Prerequisites](#prerequisites)
- [📸 Screenshots](#-screenshots)
- [📦 Installation](#-installation)
- [🚀 Special thanks to creators/contributors of used packages](#-special-thanks)
- [❤️ Contributing](#-contributing)

## ✨ Features

- **Restore GCE Snapshots**: Apply snapshots to existing disks. Currently only mysql instances are supported.
- **Restore GCE Disks**: Create new disks from snapshots. ![in progress](https://img.shields.io/badge/status-in--progress-red)
- **Restore GKE Clusters**: Restore GKE clusters from backups.![in progress](https://img.shields.io/badge/status-in--progress-red)

## Prerequisites

Before you begin, ensure you have the following installed and configured:

- [Google Cloud SDK](https://cloud.google.com/sdk/docs/install) (`gcloud`) authenticated to your account.
- Python 3.8+
- A GCP project with the necessary APIs enabled (e.g., Compute Engine API).
- Appropriate IAM permissions to manage the resources you intend to restore (e.g., `roles/compute.instanceAdmin.v1`).

## 📸 Screenshots

![](/assets/all-gif.webp)

## 📦 Installation

### Using Cargo

If you have the Rust toolchain installed, you can install `gcp-snap-crab` directly from crates.io:

```sh
cargo install gcp-snap-crab
```

### Using Homebrew (macOS)

*Coming soon! Once the project is published, you will be able to install it with:*

```sh
brew install gcp-snap-crab
```

## 🚀 Special thanks to creators/contributors of used packages

- [ratatui](https://github.com/ratatui-org/ratatui)
- [crossterm](https://github.com/crossterm-rs/crossterm)
- [serde](https://github.com/serde-rs/serde)
- [serde_json](https://github.com/serde-rs/serde_json)
- [reqwest](https://github.com/seanmonstar/reqwest)
- [anyhow](https://github.com/dtolnay/anyhow)
- [chrono](https://github.com/chronotope/chrono)
- [clap](https://github.com/clap-rs/clap)

## ❤️ Contributing

This is my second project in Rust, and I'm passionate about making it better! I welcome all forms of contributions, from feature suggestions and bug reports to code improvements and pull requests.

If you have ideas on how to improve the code, make it more idiomatic, or enhance its performance, please don't hesitate to open an issue or a pull request. Your feedback is incredibly valuable.

1. **Fork the repository.**
2. **Create a new branch** (`git checkout -b feature/your-feature-name`).
3. **Make your changes.**
4. **Commit your changes** (`git commit -m 'Add some amazing feature'`).
5. **Push to the branch** (`git push origin feature/your-feature-name`).
6. **Open a Pull Request.**
