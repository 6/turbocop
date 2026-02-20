# frozen_string_literal: true

module Turbocop
  VERSION = "0.0.1.pre"

  # Returns the path to the precompiled turbocop binary, or nil if
  # no binary is bundled (e.g. the base/fallback gem on an unsupported platform).
  def self.executable
    bin = File.expand_path("../libexec/turbocop", __dir__)
    bin if File.file?(bin) && File.executable?(bin)
  end
end
