class ProjectGuardrails < Formula
  desc "Portable repo-local guardrails bootstrap utility"
  homepage "https://github.com/firblab-blog/project-guardrails"
  version "0.1.10"
  license "MIT OR Apache-2.0"

  if OS.mac?
    if Hardware::CPU.arm?
      url "https://github.com/firblab-blog/project-guardrails/releases/download/v0.1.10/project-guardrails-v0.1.10-aarch64-apple-darwin.tar.gz"
      sha256 "REPLACE_WITH_V0_1_6_MACOS_ARM64_SHA256"
    else
      url "https://github.com/firblab-blog/project-guardrails/releases/download/v0.1.10/project-guardrails-v0.1.10-x86_64-apple-darwin.tar.gz"
      sha256 "REPLACE_WITH_V0_1_6_MACOS_X86_64_SHA256"
    end
  else
      url "https://github.com/firblab-blog/project-guardrails/releases/download/v0.1.10/project-guardrails-v0.1.10-x86_64-unknown-linux-gnu.tar.gz"
    sha256 "REPLACE_WITH_V0_1_6_LINUX_X86_64_SHA256"
  end

  def install
    bin.install "project-guardrails"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/project-guardrails --version")
  end
end
