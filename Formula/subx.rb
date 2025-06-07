class Subx < Formula
  desc "智慧字幕處理 CLI 工具"
  homepage "https://github.com/jim60105/subx-cli"
  url "https://github.com/jim60105/subx-cli/archive/v0.1.0.tar.gz"
  sha256 "YOUR_SHA256_HERE"
  license "GPL-3.0-or-later"

  depends_on "rust" => :build
  depends_on "ffmpeg" => :optional

  def install
    system "cargo", "install", *std_cargo_args
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/subx --version")
  end
end
