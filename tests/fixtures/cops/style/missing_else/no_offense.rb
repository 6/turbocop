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

# if/elsif without else — RuboCop treats elsif as having an else clause
if x > 1
  bar
elsif x < 0
  baz
end
