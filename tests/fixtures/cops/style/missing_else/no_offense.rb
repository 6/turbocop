if cond
  foo
else
  bar
end
case status
when :active
  activate
else
  deactivate
end
if x > 1
  bar
elsif x < 0
  baz
else
  qux
end

# Modifier if — no else needed
return x if condition
x = 42 if flag
puts "hello" if verbose

# unless without else — not flagged when Style/UnlessElse is enabled (default)
unless cond
  foo
end

# case..in pattern matching — never flagged
case pattern
in a
  foo
end
