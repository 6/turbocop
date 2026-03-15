a = "a"
if a == "a"
  print a
end
["a", "b"].include?(a)
a == "x" || b == "y"
a == 1
x == y

# AllowMethodComparison (default: true) — method call values are allowed
x == foo.bar || x == baz.qux
username == config.local_domain || username == config.web_domain

# Mixed == and non-== methods in || chain — not all comparisons use ==
file == "Pipfile" || file == "pyproject.toml" || file.end_with?(".in") || file.end_with?(".txt")
lower_segment == "a" || lower_segment == "alpha" || lower_segment.match?(/^(a|alpha)\d/)
normalized_path == pattern || normalized_path == normalized_pattern
a == "x" || a == "y" || a.start_with?("z")
