if condition
  statement
else
  other_statement
end

if condition
  statement
end

case x
when 1
  "one"
when 2
  "two"
end

# else with actual content
unless condition
  a
else
  b
end

# case with else that has content
case x
when 1
  "one"
else
  "default"
end

# if/elsif chain with non-empty else
if a
  1
elsif b
  2
else
  3
end
