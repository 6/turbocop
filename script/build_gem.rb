#!/usr/bin/env ruby
# frozen_string_literal: true

# Builds a turbocop gem — base (fallback) or platform-specific.
#
# Usage:
#   ruby script/build_gem.rb VERSION
#   ruby script/build_gem.rb VERSION --platform PLATFORM --binary PATH
#
# Examples:
#   ruby script/build_gem.rb 0.1.0
#   ruby script/build_gem.rb 0.1.0 --platform arm64-darwin --binary target/aarch64-apple-darwin/release/turbocop

require "optparse"
require_relative "lib/gem_builder"

options = {}
parser = OptionParser.new do |opts|
  opts.banner = "Usage: ruby #{$PROGRAM_NAME} VERSION [options]"
  opts.on("--platform PLATFORM", "Target platform (e.g. arm64-darwin)") { |v| options[:platform] = v }
  opts.on("--binary PATH", "Path to compiled binary") { |v| options[:binary] = v }
end
parser.parse!

version = ARGV[0]
unless version
  abort parser.to_s
end

if options[:platform] && !options[:binary]
  abort "Error: --binary is required when --platform is specified"
end

if options[:binary] && !options[:platform]
  abort "Error: --platform is required when --binary is specified"
end

if options[:binary] && !File.file?(options[:binary])
  abort "Error: binary not found: #{options[:binary]}"
end

dest = GemBuilder.new(
  version: version,
  platform: options[:platform],
  binary_path: options[:binary],
).build
puts "Built: #{dest}"
