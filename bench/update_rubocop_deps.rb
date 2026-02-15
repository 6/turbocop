#!/usr/bin/env ruby
# frozen_string_literal: true

# Update rubocop gem versions in bench repos to match our vendor submodules.
#
# This script:
# 1. Reads the version from each vendor submodule's version.rb
# 2. Updates each bench repo's Gemfile to pin those exact versions
# 3. Runs `bundle update` to install the new versions
#
# For repos that use meta-gems (e.g., Discourse uses rubocop-discourse which
# transitively depends on rubocop-rails, rubocop-rspec, etc.), the script
# runs `bundle update` on the rubocop gems to pull in the latest compatible
# versions allowed by the meta-gem's constraints.
#
# Usage:
#   ruby bench/update_rubocop_deps.rb          # update all bench repos
#   ruby bench/update_rubocop_deps.rb --dry-run # show what would change

require "pathname"
require "fileutils"

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

def check_submodule_tags
  puts "Checking vendor submodule tags..."
  all_ok = true

  VERSION_FILES.each_key do |gem_name|
    dir = VENDOR_DIR.join(gem_name)
    next unless dir.exist?

    describe = `cd #{dir} && git describe --tags --exact-match 2>/dev/null`.strip
    if describe.empty?
      current = `cd #{dir} && git describe --tags 2>/dev/null`.strip
      current = `cd #{dir} && git log --oneline -1`.strip if current.empty?
      version = read_vendor_version(gem_name)
      expected_tag = "v#{version}"
      puts "  #{gem_name}: NOT on a release tag (at: #{current})"
      puts "    Fix: cd vendor/#{gem_name} && git fetch --tags && git checkout #{expected_tag}"
      all_ok = false
    else
      puts "  #{gem_name}: #{describe} (OK)"
    end
  end

  all_ok
end

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
    # Match gem lines like:
    #   gem 'rubocop', require: false
    #   gem "rubocop-rails", "~> 2.26", require: false
    #   gem "rubocop-rails", require: false
    # Only match if the gem is directly listed (not a transitive dep)
    pattern = /^(\s*gem\s+['"]#{Regexp.escape(gem_name)}['"])(?:,\s*['"][^'"]*['"])?(.*)/

    content = content.gsub(pattern) do
      prefix = $1
      suffix = $2
      new_line = "#{prefix}, \"~> #{version}\"#{suffix}"
      changed = true if new_line != "#{prefix}#{suffix}"
      new_line
    end
  end

  if changed
    if dry_run
      puts "  Would update #{gemfile}"
      original.lines.zip(content.lines).each_with_index do |(old_line, new_line), i|
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

def bundle_update(repo_path, versions, dry_run:)
  # Update all rubocop gems AND their companion gems that must be compatible
  # (rubocop-rspec_rails and rubocop-capybara use APIs that change between versions)
  companion_gems = %w[rubocop-rspec_rails rubocop-capybara rubocop-factory_bot]
  gems_to_update = (versions.keys + companion_gems).uniq

  if dry_run
    puts "  Would run: bundle update #{gems_to_update.join(' ')}"
    return
  end

  puts "  Running bundle update #{gems_to_update.join(' ')}..."
  Dir.chdir(repo_path) do
    system("bundle", "update", *gems_to_update, exception: false)
  end
end

def verify_versions(repo_path, versions)
  puts "  Installed versions:"
  all_ok = true
  versions.each do |gem_name, target|
    installed = `cd #{repo_path} && bundle info #{gem_name} 2>/dev/null`.strip
    ver_match = installed.match(/\((\d+\.\d+\.\d+)\)/)
    if ver_match
      status = ver_match[1] == target ? "OK" : "MISMATCH (wanted #{target}, got #{ver_match[1]})"
      all_ok = false if ver_match[1] != target
      puts "    #{gem_name}: #{ver_match[1]} #{status}"
    else
      puts "    #{gem_name}: not installed"
      all_ok = false
    end
  end
  all_ok
end

# --- Main ---

dry_run = ARGV.include?("--dry-run")
puts "Mode: #{dry_run ? 'DRY RUN' : 'LIVE'}\n\n"

puts "Reading vendor submodule versions..."
versions = read_vendor_versions
versions.each { |gem, ver| puts "  #{gem}: #{ver}" }
puts

tags_ok = check_submodule_tags
puts

unless REPOS_DIR.exist?
  puts "No bench repos directory. Run `cargo run --release --bin bench_rblint -- setup` first."
  exit 1
end

repos = Dir.children(REPOS_DIR).select { |d| (REPOS_DIR + d).directory? }.sort
if repos.empty?
  puts "No bench repos found in #{REPOS_DIR}."
  exit 1
end

repos.each do |repo_name|
  repo_path = REPOS_DIR.join(repo_name)
  puts "Updating #{repo_name}..."

  # Pin versions in Gemfile where possible
  update_gemfile(repo_path, versions, dry_run: dry_run)

  # Always run bundle update to pull in latest compatible versions
  # (handles transitive deps from meta-gems like rubocop-discourse)
  bundle_update(repo_path, versions, dry_run: dry_run)

  # Verify
  verify_versions(repo_path, versions) unless dry_run

  puts
end

puts "Done! Run conformance check: cargo run --release --bin bench_rblint -- conform"
