if condition
  foo
else
  bar
end
if x
  y
elsif z
  w
end
# Single-line if/then/else — not flagged
if a then b else c end
# Single-line with expressions
if cond then '-' else '+' end
# then-style with single else expression (not multi-line)
if something then test
else something_else
end
# then-style with single else expression
if val then puts "true"
else puts "false"
end
# Multi-line then with single else body
if x <= y
then y
else z
end
