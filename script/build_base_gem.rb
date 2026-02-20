#!/usr/bin/env ruby
# frozen_string_literal: true

# Builds the base (fallback) gem without a precompiled binary.
# Users on unsupported platforms get a helpful error message.
#
# Usage: ruby script/build_base_gem.rb VERSION
#
# Produces: turbocop-VERSION.gem in the current directory.

require "fileutils"
require "tmpdir"

version = ARGV[0]
unless version
  abort "Usage: ruby #{$PROGRAM_NAME} VERSION"
end

gem_source = File.expand_path("../gems/turbocop", __dir__)

Dir.mktmpdir("turbocop-gem-") do |tmpdir|
  # Copy gem source files (no libexec/ — no binary)
  FileUtils.cp_r(File.join(gem_source, "lib"), tmpdir)
  FileUtils.cp_r(File.join(gem_source, "exe"), tmpdir)
  FileUtils.cp(File.join(gem_source, "turbocop.gemspec"), tmpdir)

  # Patch version
  version_file = File.join(tmpdir, "lib", "turbocop.rb")
  content = File.read(version_file)
  content.sub!(/VERSION = ".*"/, %(VERSION = "#{version}"))
  File.write(version_file, content)

  # Build the gem
  Dir.chdir(tmpdir) do
    system("gem", "build", "turbocop.gemspec", exception: true)
  end

  # Move the .gem file to the project root
  gem_file = Dir.glob(File.join(tmpdir, "turbocop-*.gem")).first
  dest = File.join(Dir.pwd, File.basename(gem_file))
  FileUtils.mv(gem_file, dest)
  puts "Built: #{dest}"
end
