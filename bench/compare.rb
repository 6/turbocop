#!/usr/bin/env ruby
# frozen_string_literal: true

# Compare nitrocop JSON output against rubocop JSON output.
# Usage: compare.rb [--json out.json] <nitrocop.json> <rubocop.json> [covered-cops.txt] [repo-dir]

require "json"

# Parse --json flag
json_output_file = nil
args = ARGV.dup
if (idx = args.index("--json"))
  args.delete_at(idx)
  json_output_file = args.delete_at(idx)
end

nitrocop_file, rubocop_file, covered_cops_file, repo_dir = args
unless nitrocop_file && rubocop_file
  abort "Usage: compare.rb [--json out.json] <nitrocop.json> <rubocop.json> [covered-cops.txt] [repo-dir]"
end

# Load covered cops list (one per line from --list-cops output)
covered = if covered_cops_file && File.exist?(covered_cops_file)
            Set.new(File.readlines(covered_cops_file).map(&:strip).reject(&:empty?))
          end

# Path normalization: strip repo_dir prefix from nitrocop paths so both
# tools use paths relative to repo root (rubocop runs from repo dir)
repo_prefix = repo_dir ? "#{repo_dir.chomp("/")}/" : nil

def normalize_path(path, prefix)
  return path unless prefix
  path.delete_prefix(prefix)
end

# Parse nitrocop JSON (flat: { offenses: [ { path, line, cop_name } ] })
nitrocop_data = JSON.parse(File.read(nitrocop_file))
nitrocop_counts = Hash.new(0)
nitrocop_data["offenses"].each do |o|
  path = normalize_path(o["path"], repo_prefix)
  nitrocop_counts[[path, o["line"], o["cop_name"]]] += 1
end

# Parse rubocop JSON (nested: { files: [ { path, offenses: [ { location: { start_line }, cop_name } ] } ] })
rubocop_data = JSON.parse(File.read(rubocop_file))
rubocop_counts = Hash.new(0)
rubocop_data["files"].each do |file_entry|
  path = file_entry["path"]
  (file_entry["offenses"] || []).each do |o|
    cop = o["cop_name"]
    # Filter to only cops nitrocop covers
    next if covered && !covered.include?(cop)
    line = o.dig("location", "start_line") || o.dig("location", "line")
    rubocop_counts[[path, line, cop]] += 1
  end
end

# Compare using counter arithmetic (multiset)
all_keys = (nitrocop_counts.keys + rubocop_counts.keys).uniq
n_matches = 0
n_fp = 0
n_fn = 0
per_cop = Hash.new { |h, k| h[k] = {fp: 0, fn: 0, match: 0} }

all_keys.each do |key|
  nc = nitrocop_counts[key]
  rc = rubocop_counts[key]
  matched = [nc, rc].min
  excess = [nc - rc, 0].max
  deficit = [rc - nc, 0].max
  cop = key[2]

  n_matches += matched
  per_cop[cop][:match] += matched

  if excess > 0
    n_fp += excess
    per_cop[cop][:fp] += excess
  end
  if deficit > 0
    n_fn += deficit
    per_cop[cop][:fn] += deficit
  end
end

nitrocop_total = nitrocop_counts.values.sum
rubocop_total = rubocop_counts.values.sum
total = n_matches + n_fp + n_fn
match_rate = total.zero? ? 100.0 : (n_matches.to_f / total * 100)

puts "=== Conformance Report ==="
puts "  nitrocop offenses:  #{nitrocop_total}"
puts "  rubocop offenses: #{rubocop_total} (filtered to covered cops)"
puts "  matches:          #{n_matches}"
puts "  false positives:  #{n_fp} (nitrocop only)"
puts "  false negatives:  #{n_fn} (rubocop only)"
puts "  match rate:       #{"%.1f" % match_rate}%"
puts ""

# Per-cop breakdown (only cops with differences)
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
    nitrocop_count: nitrocop_total,
    rubocop_count: rubocop_total,
    matches: n_matches,
    false_positives: n_fp,
    false_negatives: n_fn,
    match_rate: match_rate.round(1),
    per_cop: per_cop.transform_values { |v| {match: v[:match], fp: v[:fp], fn: v[:fn]} }
  }
  File.write(json_output_file, JSON.pretty_generate(report))
end

# Exit with non-zero if there are divergences (useful for CI)
exit(divergent.empty? ? 0 : 1)
