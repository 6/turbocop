#!/usr/bin/env ruby
# frozen_string_literal: true

# Builds a platform-specific gem containing the precompiled turbocop binary.
#
# Usage: ruby script/build_platform_gem.rb VERSION PLATFORM BINARY_PATH
#
# Example: ruby script/build_platform_gem.rb 0.1.0 arm64-darwin target/aarch64-apple-darwin/release/turbocop
#
# Produces: turbocop-VERSION-PLATFORM.gem in the current directory.

require "fileutils"
require "tmpdir"

version, platform, binary_path = ARGV
unless version && platform && binary_path
  abort "Usage: ruby #{$PROGRAM_NAME} VERSION PLATFORM BINARY_PATH"
end

unless File.file?(binary_path)
  abort "Binary not found: #{binary_path}"
end

gem_source = File.expand_path("../gems/turbocop", __dir__)

Dir.mktmpdir("turbocop-gem-") do |tmpdir|
  # Copy gem source files
  FileUtils.cp_r(File.join(gem_source, "lib"), tmpdir)
  FileUtils.cp_r(File.join(gem_source, "exe"), tmpdir)

  # Copy the compiled binary into libexec/
  libexec = File.join(tmpdir, "libexec")
  FileUtils.mkdir_p(libexec)
  FileUtils.cp(binary_path, File.join(libexec, "turbocop"))
  FileUtils.chmod(0o755, File.join(libexec, "turbocop"))

  # Patch version
  version_file = File.join(tmpdir, "lib", "turbocop.rb")
  content = File.read(version_file)
  content.sub!(/VERSION = ".*"/, %(VERSION = "#{version}"))
  File.write(version_file, content)

  # Generate platform-specific gemspec
  gemspec = <<~RUBY
    # frozen_string_literal: true

    require_relative "lib/turbocop"

    Gem::Specification.new do |spec|
      spec.name     = "turbocop"
      spec.version  = Turbocop::VERSION
      spec.platform = "#{platform}"
      spec.authors  = ["6"]
      spec.email    = ["me@peterbrowne.com"]

      spec.summary     = "Fast Ruby linter targeting RuboCop compatibility"
      spec.description = "A Ruby linter written in Rust that reads your existing .rubocop.yml " \\
                         "and runs 900+ cops. Dramatically faster than RuboCop. " \\
                         "Platform variant: #{platform}."
      spec.homepage    = "https://github.com/6/turbocop"
      spec.license     = "MIT"

      spec.required_ruby_version = ">= 3.1.0"

      spec.metadata["source_code_uri"] = spec.homepage
      spec.metadata["changelog_uri"]   = "\#{spec.homepage}/releases"

      spec.files       = Dir["lib/**/*", "exe/**/*", "libexec/**/*"]
      spec.bindir      = "exe"
      spec.executables = ["turbocop"]
    end
  RUBY
  File.write(File.join(tmpdir, "turbocop.gemspec"), gemspec)

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
