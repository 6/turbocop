#!/usr/bin/env ruby
# frozen_string_literal: true

# Usage: ruby bench/update_rubocop_deps.rb [--dry-run]
#
# Updates bench repo rubocop gem versions to match vendor submodules.
# Run from the rblint project root.
#
# This script:
# 1. Verifies vendor submodules are on proper release tags
# 2. Reads the version from each vendor submodule's version.rb
# 3. For each bench repo:
#    a. Updates .rubocop.yml require: -> plugins: if rubocop >= 1.84
#    b. Updates the Gemfile to pin rubocop gem versions
#    c. Fixes Ruby version mismatches in Gemfile
#    d. Runs bundle update to install the new versions
#    e. Verifies the installed versions match

require "fileutils"
require "open3"
require "pathname"
require "yaml"

ROOT = Pathname.new(__dir__).join("..").expand_path
VENDOR_DIR = ROOT.join("vendor")
REPOS_DIR = ROOT.join("bench", "repos")

# Map of gem name -> path to version.rb relative to vendor/<gem>/
VERSION_FILES = {
  "rubocop"             => "lib/rubocop/version.rb",
  "rubocop-rails"       => "lib/rubocop/rails/version.rb",
  "rubocop-rspec"       => "lib/rubocop/rspec/version.rb",
  "rubocop-performance" => "lib/rubocop/performance/version.rb",
}.freeze

# Rubocop plugins that should use plugins: instead of require: in >= 1.84
PLUGIN_GEMS = %w[
  rubocop-rails
  rubocop-rspec
  rubocop-rspec_rails
  rubocop-performance
  rubocop-capybara
  rubocop-factory_bot
  rubocop-minitest
  rubocop-packaging
  rubocop-rake
  rubocop-thread_safety
  rubocop-md
].freeze

# Companion gems that should also be updated when present
COMPANION_GEMS = %w[rubocop-rspec_rails rubocop-capybara rubocop-factory_bot].freeze

# ---------------------------------------------------------------------------
# Version reading
# ---------------------------------------------------------------------------

def read_vendor_version(gem_name)
  version_file = VENDOR_DIR.join(gem_name, VERSION_FILES[gem_name])
  unless version_file.exist?
    warn "  WARNING: #{version_file} not found"
    return nil
  end

  content = version_file.read
  # Match patterns like: STRING = '1.84.2' or VERSION = '3.9.0'
  if content =~ /(?:STRING|VERSION)\s*=\s*['"](\d+\.\d+\.\d+)['"]/
    $1
  else
    warn "  WARNING: Could not parse version from #{version_file}"
    nil
  end
end

def read_vendor_versions
  versions = {}
  VERSION_FILES.each_key do |gem_name|
    dir = VENDOR_DIR.join(gem_name)
    unless dir.exist?
      warn "  Skipping #{gem_name}: vendor/#{gem_name} not found"
      next
    end
    ver = read_vendor_version(gem_name)
    versions[gem_name] = ver if ver
  end
  versions
end

# ---------------------------------------------------------------------------
# Submodule tag verification
# ---------------------------------------------------------------------------

def check_submodule_tags
  puts "Checking vendor submodule tags..."
  all_ok = true

  VERSION_FILES.each_key do |gem_name|
    dir = VENDOR_DIR.join(gem_name)
    next unless dir.exist?

    describe, = Open3.capture2("git", "describe", "--tags", "--exact-match", chdir: dir.to_s)
    describe = describe.strip

    if describe.empty?
      current, = Open3.capture2("git", "describe", "--tags", chdir: dir.to_s)
      current = current.strip
      if current.empty?
        current, = Open3.capture2("git", "log", "--oneline", "-1", chdir: dir.to_s)
        current = current.strip
      end
      version = read_vendor_version(gem_name)
      expected_tag = "v#{version}"
      puts "  WARNING: #{gem_name} NOT on a release tag (at: #{current})"
      puts "    Fix: cd vendor/#{gem_name} && git fetch --tags && git checkout #{expected_tag}"
      all_ok = false
    else
      puts "  #{gem_name}: #{describe} (OK)"
    end
  end

  all_ok
end

# ---------------------------------------------------------------------------
# .rubocop.yml require: -> plugins: conversion
# ---------------------------------------------------------------------------

def convert_require_to_plugins(repo_path, rubocop_version, dry_run:)
  config_file = repo_path.join(".rubocop.yml")
  return false unless config_file.exist?

  # Only convert if rubocop >= 1.84 (which broke require: for plugin gems)
  major, minor, = rubocop_version.split(".").map(&:to_i)
  return false unless major > 1 || (major == 1 && minor >= 84)

  content = config_file.read
  original = content.dup

  # Parse YAML to understand the structure, but do text-level replacement
  # to preserve formatting and comments
  begin
    yaml = YAML.safe_load(content, permitted_classes: [Regexp, Symbol]) || {}
  rescue StandardError => e
    puts "  WARNING: Could not parse #{config_file}: #{e.message}"
    return false
  end

  requires = yaml["require"]
  return false unless requires.is_a?(Array)

  # Separate plugin gems from non-plugin requires (e.g., local .rb files)
  plugin_requires = requires.select { |r| PLUGIN_GEMS.include?(r) }
  return false if plugin_requires.empty?

  non_plugin_requires = requires - plugin_requires

  existing_plugins = yaml["plugins"]
  existing_plugins = existing_plugins.is_a?(Array) ? existing_plugins : []

  # Merge plugin requires into plugins list (avoid duplicates)
  new_plugins = (existing_plugins + plugin_requires).uniq

  changed = false

  # Build the replacement for the require: block
  # We need to handle multi-line YAML array syntax
  if non_plugin_requires.empty?
    # Remove the entire require: block (only had plugin gems)
    # Match "require:\n  - gem1\n  - gem2\n" pattern
    require_pattern = /^require:\s*\n((?:\s+-\s+(?:#{plugin_requires.map { |g| Regexp.escape(g) }.join("|")})\s*\n)+)/m
    if content.match?(require_pattern)
      content = content.sub(require_pattern, "")
      changed = true
    end
  else
    # Keep require: block but remove plugin gems from it
    plugin_requires.each do |gem_name|
      line_pattern = /^\s+-\s+#{Regexp.escape(gem_name)}\s*\n/
      if content.match?(line_pattern)
        content = content.sub(line_pattern, "")
        changed = true
      end
    end
  end

  # Add or update plugins: block
  if existing_plugins.empty? && changed
    # No existing plugins: block, add one at the top (or after inherit_from/inherit_mode)
    plugins_yaml = "plugins:\n" + new_plugins.map { |p| "  - #{p}" }.join("\n") + "\n\n"

    # Try to insert after inherit_from/inherit_mode blocks, or at the beginning
    if content.match?(/^inherit_mode:.*?\n(?:\s+.*\n)*/m)
      # Insert after inherit_mode block
      content = content.sub(/(^inherit_mode:.*?\n(?:\s+.*\n)*\n?)/m) { "#{$1}\n#{plugins_yaml}" }
    elsif content.match?(/^inherit_from:.*?\n(?:\s+-\s+.*\n)*/m)
      content = content.sub(/(^inherit_from:.*?\n(?:\s+-\s+.*\n)*\n?)/m) { "#{$1}\n#{plugins_yaml}" }
    elsif content.start_with?("---\n")
      content = content.sub("---\n", "---\n\n#{plugins_yaml}")
    else
      content = plugins_yaml + content
    end
  elsif changed
    # Existing plugins: block -- replace it with merged list
    plugins_block_pattern = /^plugins:\s*\n(?:\s+-\s+\S+\s*\n)*/m
    new_plugins_block = "plugins:\n" + new_plugins.map { |p| "  - #{p}" }.join("\n") + "\n"
    content = content.sub(plugins_block_pattern, new_plugins_block)
  end

  if changed && content != original
    if dry_run
      puts "  Would convert require: -> plugins: in #{config_file}"
      plugin_requires.each do |gem_name|
        puts "    Moving #{gem_name} from require: to plugins:"
      end
    else
      config_file.write(content)
      puts "  Converted require: -> plugins: in #{config_file}"
      plugin_requires.each do |gem_name|
        puts "    Moved #{gem_name} from require: to plugins:"
      end
    end
    true
  else
    false
  end
end

# ---------------------------------------------------------------------------
# Gemfile modification
# ---------------------------------------------------------------------------

def update_gemfile(repo_path, versions, dry_run:)
  gemfile = repo_path.join("Gemfile")
  unless gemfile.exist?
    warn "  No Gemfile found in #{repo_path}"
    return false
  end

  content = gemfile.read
  original = content.dup
  changed = false

  versions.each do |gem_name, version|
    # Pattern 1: gem with existing version pin
    # e.g., gem 'rubocop', '~> 1.66', require: false
    # e.g., gem "rubocop-rails", "2.30.0", require: false
    pattern_with_version = /^(\s*gem\s+['"]#{Regexp.escape(gem_name)}['"])\s*,\s*['"][^'"]*['"](.*)/

    # Pattern 2: gem without version pin
    # e.g., gem 'rubocop', require: false
    # e.g., gem "rubocop"
    pattern_without_version = /^(\s*gem\s+['"]#{Regexp.escape(gem_name)}['"])(\s*$|\s*,\s*(?!['"]).*)/

    if content.match?(pattern_with_version)
      content = content.gsub(pattern_with_version) do
        prefix = $1
        suffix = $2
        new_line = "#{prefix}, '#{version}'#{suffix}"
        changed = true
        new_line
      end
    elsif content.match?(pattern_without_version)
      content = content.gsub(pattern_without_version) do
        prefix = $1
        suffix = $2
        new_line = "#{prefix}, '#{version}'#{suffix}"
        changed = true
        new_line
      end
    end
  end

  if changed
    if dry_run
      puts "  Would update #{gemfile}"
      original.lines.zip(content.lines).each_with_index do |(old_line, new_line), _i|
        if old_line != new_line
          puts "    - #{old_line&.strip}"
          puts "    + #{new_line&.strip}"
        end
      end
    else
      gemfile.write(content)
      puts "  Updated #{gemfile}"
    end
  else
    puts "  #{gemfile}: no direct gem lines to update"
  end

  changed
end

# ---------------------------------------------------------------------------
# Ruby version mismatch fix
# ---------------------------------------------------------------------------

def fix_ruby_version(repo_path, dry_run:)
  gemfile = repo_path.join("Gemfile")
  return false unless gemfile.exist?

  content = gemfile.read

  # Match exact ruby version pins like: ruby '3.4.5' or ruby "3.4.5"
  # Don't touch constraint-style versions like ruby '>= 3.1.0'
  pattern = /^(ruby\s+['"])(\d+\.\d+\.\d+)(['"])/

  return false unless content.match?(pattern)

  match = content.match(pattern)
  pinned_version = match[2]

  # Get the system Ruby version
  system_ruby, = Open3.capture2("ruby", "-e", "puts RUBY_VERSION")
  system_ruby = system_ruby.strip

  return false if system_ruby.empty?
  return false if pinned_version == system_ruby

  # Only fix if the major.minor matches (e.g., 3.4.5 -> 3.4.8, not 3.3.0 -> 3.4.8)
  pinned_parts = pinned_version.split(".")
  system_parts = system_ruby.split(".")
  return false unless pinned_parts[0] == system_parts[0] && pinned_parts[1] == system_parts[1]

  if dry_run
    puts "  Would update Ruby version: #{pinned_version} -> #{system_ruby}"
  else
    content = content.sub(pattern, "\\1#{system_ruby}\\3")
    gemfile.write(content)
    puts "  Updated Ruby version: #{pinned_version} -> #{system_ruby}"
  end

  true
end

# ---------------------------------------------------------------------------
# Bundle update
# ---------------------------------------------------------------------------

def gems_in_gemfile(repo_path, gem_names)
  gemfile = repo_path.join("Gemfile")
  return [] unless gemfile.exist?

  content = gemfile.read
  gem_names.select { |name| content.match?(/^\s*gem\s+['"]#{Regexp.escape(name)}['"]/) }
end

def bundle_update(repo_path, versions, dry_run:)
  # Only update gems that are actually in the Gemfile
  all_gems = (versions.keys + COMPANION_GEMS).uniq
  present_gems = gems_in_gemfile(repo_path, all_gems)

  if present_gems.empty?
    puts "  No rubocop gems found in Gemfile to update"
    return
  end

  if dry_run
    puts "  Would run: bundle update #{present_gems.join(' ')}"
    return
  end

  puts "  Running bundle update #{present_gems.join(' ')}..."
  output, status = Open3.capture2e("bundle", "update", *present_gems, chdir: repo_path.to_s)
  unless status.success?
    puts "  WARNING: bundle update failed:"
    output.lines.last(10).each { |l| puts "    #{l}" }
  end
end

# ---------------------------------------------------------------------------
# Verification
# ---------------------------------------------------------------------------

def verify_versions(repo_path, versions)
  puts "  Verifying installed versions:"
  all_ok = true

  # Only verify gems present in the Gemfile
  present_gems = gems_in_gemfile(repo_path, versions.keys)

  present_gems.each do |gem_name|
    target = versions[gem_name]
    output, = Open3.capture2("bundle", "info", gem_name, chdir: repo_path.to_s)
    ver_match = output.match(/\((\d+\.\d+\.\d+)/)
    if ver_match
      installed = ver_match[1]
      if installed == target
        puts "    #{gem_name}: #{installed} (OK)"
      else
        puts "    #{gem_name}: #{installed} (MISMATCH - wanted #{target})"
        all_ok = false
      end
    else
      puts "    #{gem_name}: not installed"
      all_ok = false
    end
  end

  # Also check rubocop --version
  rubocop_output, status = Open3.capture2e("bundle", "exec", "rubocop", "--version", chdir: repo_path.to_s)
  if status.success?
    puts "    rubocop --version: #{rubocop_output.strip}"
  else
    puts "    WARNING: rubocop --version failed"
    all_ok = false
  end

  all_ok
end

# ---------------------------------------------------------------------------
# Summary
# ---------------------------------------------------------------------------

def print_summary(results)
  puts "=" * 60
  puts "Summary"
  puts "=" * 60

  results.each do |repo_name, status|
    emoji = status[:ok] ? "OK" : "ISSUES"
    changes = status[:changes].empty? ? "no changes" : status[:changes].join(", ")
    puts "  #{repo_name}: [#{emoji}] #{changes}"
  end
end

# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

dry_run = ARGV.include?("--dry-run")
puts "Mode: #{dry_run ? 'DRY RUN' : 'LIVE'}"
puts

# 1. Read vendor versions
puts "Reading vendor submodule versions..."
versions = read_vendor_versions
if versions.empty?
  puts "ERROR: No vendor versions found. Are submodules initialized?"
  puts "  Run: git submodule update --init"
  exit 1
end
versions.each { |gem, ver| puts "  #{gem}: #{ver}" }
puts

# 2. Verify submodule tags
tags_ok = check_submodule_tags
unless tags_ok
  puts
  puts "WARNING: Some submodules are not on release tags."
  puts "  This may indicate the submodules need to be updated."
  puts "  Continuing anyway..."
end
puts

# 3. Process bench repos
unless REPOS_DIR.exist?
  puts "No bench repos directory at #{REPOS_DIR}."
  puts "Run `cargo run --release --bin bench_rblint -- setup` first."
  exit 1
end

repos = Dir.children(REPOS_DIR.to_s)
             .select { |d| File.directory?(REPOS_DIR.join(d)) }
             .sort

if repos.empty?
  puts "No bench repos found in #{REPOS_DIR}."
  exit 1
end

rubocop_version = versions["rubocop"] || "0.0.0"
results = {}

repos.each do |repo_name|
  repo_path = REPOS_DIR.join(repo_name)
  puts "-" * 60
  puts "Processing: #{repo_name}"
  puts "-" * 60

  changes = []
  ok = true

  # a. Convert require: -> plugins: if needed
  if convert_require_to_plugins(repo_path, rubocop_version, dry_run: dry_run)
    changes << "require->plugins"
  end

  # b. Fix Ruby version mismatch
  if fix_ruby_version(repo_path, dry_run: dry_run)
    changes << "ruby-version"
  end

  # c. Update Gemfile version pins
  if update_gemfile(repo_path, versions, dry_run: dry_run)
    changes << "gemfile-pins"
  end

  # d. Run bundle update
  bundle_update(repo_path, versions, dry_run: dry_run)

  # e. Verify
  unless dry_run
    ok = verify_versions(repo_path, versions)
  end

  results[repo_name] = { ok: ok, changes: changes }
  puts
end

# 4. Print summary
print_summary(results)
puts
puts "Done!"
unless dry_run
  puts "Next step: cargo run --release --bin bench_rblint -- conform"
end
