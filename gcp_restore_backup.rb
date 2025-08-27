# This is a Homebrew formula for Pomodorust.
# You would place this file in your Homebrew tap repository.
# For example: your-github-username/homebrew-tap/Formula/gcp_restore_backup.rb
class GcpRestoreBackup < Formula
  desc "A tool for restoring Google Cloud Platform (GCP) resources from backups"
  homepage "https://github.com/ruteckimikolaj/gcp-snap-crab"
  version "1.0.0"

  # This section provides different binaries for different macOS architectures.
  if Hardware::CPU.intel?
    # For Intel Macs
    url "https://github.com/ruteckimikolaj/gcp-snap-crab/releases/download/v1.0.0/gcp-snap-crab-macos-x86_64.tar.gz"
    sha256 "..." # TODO: Update with the SHA256 hash of the tarball
  else
    # For Apple Silicon Macs
    url "https://github.com/ruteckimikolaj/gcp-snap-crab/releases/download/v1.0.0/gcp-snap-crab-macos-aarch64.tar.gz"
    sha256 "..." # TODO: Update with the SHA256 hash of the tarball
  end

  def install
    # The binary is installed directly from the tarball.
    bin.install "gcp-snap-crab"
  end

  # Optional: Add tests to verify the installation.
  test do
    system "#{bin}/gcp-snap-crab", "--version"
  end
end
