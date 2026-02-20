#!/usr/bin/env ruby
# frozen_string_literal: true

# Compare turbocop JSON output against rubocop JSON output.
# Usage: compare.rb [--json out.json] <turbocop.json> <rubocop.json> [covered-cops.txt] [repo-dir]

require "json"
require "set"

# Parse --json flag
json_output_file = nil
args = ARGV.dup
if (idx = args.index("--json"))
  args.delete_at(idx)
  json_output_file = args.delete_at(idx)
end

turbocop_file, rubocop_file, covered_cops_file, repo_dir = args
unless turbocop_file && rubocop_file
  abort "Usage: compare.rb [--json out.json] <turbocop.json> <rubocop.json> [covered-cops.txt] [repo-dir]"
end

# Load covered cops list (one per line from --list-cops output)
covered = if covered_cops_file && File.exist?(covered_cops_file)
            File.readlines(covered_cops_file).map(&:strip).reject(&:empty?).to_set
          end

# Path normalization: strip repo_dir prefix from turbocop paths so both
# tools use paths relative to repo root (rubocop runs from repo dir)
repo_prefix = repo_dir ? "#{repo_dir.chomp("/")}/" : nil

def normalize_path(path, prefix)
  return path unless prefix
  path.delete_prefix(prefix)
end

# Parse turbocop JSON (flat: { offenses: [ { path, line, cop_name } ] })
turbocop_data = JSON.parse(File.read(turbocop_file))
turbocop_offenses = Set.new
turbocop_data["offenses"].each do |o|
  path = normalize_path(o["path"], repo_prefix)
  turbocop_offenses << [path, o["line"], o["cop_name"]]
end

# Parse rubocop JSON (nested: { files: [ { path, offenses: [ { location: { start_line }, cop_name } ] } ] })
rubocop_data = JSON.parse(File.read(rubocop_file))
rubocop_offenses = Set.new
rubocop_data["files"].each do |file_entry|
  path = file_entry["path"]
  (file_entry["offenses"] || []).each do |o|
    cop = o["cop_name"]
    # Filter to only cops turbocop covers
    next if covered && !covered.include?(cop)
    line = o.dig("location", "start_line") || o.dig("location", "line")
    rubocop_offenses << [path, line, cop]
  end
end

# Compare
false_positives = turbocop_offenses - rubocop_offenses
false_negatives = rubocop_offenses - turbocop_offenses
matches = turbocop_offenses & rubocop_offenses
total = (turbocop_offenses | rubocop_offenses).size
match_rate = total.zero? ? 100.0 : (matches.size.to_f / total * 100)

puts "=== Conformance Report ==="
puts "  turbocop offenses:  #{turbocop_offenses.size}"
puts "  rubocop offenses: #{rubocop_offenses.size} (filtered to covered cops)"
puts "  matches:          #{matches.size}"
puts "  false positives:  #{false_positives.size} (turbocop only)"
puts "  false negatives:  #{false_negatives.size} (rubocop only)"
puts "  match rate:       #{"%.1f" % match_rate}%"
puts ""

# Per-cop breakdown (only cops with differences)
per_cop = Hash.new { |h, k| h[k] = {fp: 0, fn: 0, match: 0} }
false_positives.each { |_, _, cop| per_cop[cop][:fp] += 1 }
false_negatives.each { |_, _, cop| per_cop[cop][:fn] += 1 }
matches.each { |_, _, cop| per_cop[cop][:match] += 1 }

divergent = per_cop.select { |_, v| v[:fp] > 0 || v[:fn] > 0 }
  .sort_by { |_, v| -(v[:fp] + v[:fn]) }

if divergent.empty?
  puts "All cops match perfectly!"
else
  puts "Divergent cops (sorted by total differences):"
  puts "  #{"Cop".ljust(45)} #{"Match".rjust(6)} #{"FP".rjust(6)} #{"FN".rjust(6)}"
  puts "  #{"-" * 63}"
  divergent.each do |cop, counts|
    puts "  #{cop.ljust(45)} #{counts[:match].to_s.rjust(6)} " \
         "#{counts[:fp].to_s.rjust(6)} #{counts[:fn].to_s.rjust(6)}"
  end
end

# Write machine-readable JSON for report.rb
if json_output_file
  report = {
    turbocop_count: turbocop_offenses.size,
    rubocop_count: rubocop_offenses.size,
    matches: matches.size,
    false_positives: false_positives.size,
    false_negatives: false_negatives.size,
    match_rate: match_rate.round(1),
    per_cop: per_cop.transform_values { |v| {match: v[:match], fp: v[:fp], fn: v[:fn]} }
  }
  File.write(json_output_file, JSON.pretty_generate(report))
end

# Exit with non-zero if there are divergences (useful for CI)
exit(divergent.empty? ? 0 : 1)
