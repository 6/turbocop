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

# Both comparison values are local variables (simple_double_comparison)
still_active = 259
exit_status = 0
while exit_status == 0 || exit_status == still_active
  break
end

# Block params as comparison values — both sides are lvars
value = 1
tuples.select { |x, y| value == x || value == y }

# Two local variables compared (lvar == lvar is not flagged)
a = "a"
b = "b"
if a == b || b == a
  print a
end

# Comparison value is a local variable, not a literal
prototype_path = "sig/prototype"
generated_path = "sig/generated"
paths.reject { |path| path == prototype_path || path == generated_path }

# Later repeated subchains after a different variable are not flagged.
a = 1
b = 2
a == 1 || b == 2 || b == 3

# A non-comparison prefix prevents flagging later repeated comparisons.
a = 1
if 1 == 1 || 1 == 2 || a == 3 || a == 4
  nil
end

# Only one literal comparison value (below threshold)
actual_height = 100
expected_height = 200
unless actual_height == expected_height || actual_height == 0
  nil
end
