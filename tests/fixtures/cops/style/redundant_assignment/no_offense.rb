def func
  something
end
def bar
  x = compute
  transform(x)
end
def baz
  result = a + b
  log(result)
  result
end
x = 1
x

# Ensure block present — not flagged
def with_ensure
  1
  x = 2
  x
ensure
  3
end

# Empty method body
def empty_method
end

# Empty if body
def empty_if
  if x
  elsif y
  else
  end
end

# Empty when branches
def empty_when
  case x
  when y then 1
  when z # do nothing
  else
    3
  end
end

# Empty in branches
def empty_in
  case x
  in y then 1
  in z # do nothing
  else
    3
  end
end

# Modifier if — not checked
def modifier_if
  x = 1
  x if condition
end

# Ternary — not checked
def ternary_method
  condition ? foo : bar
end

# op_asgn should not be flagged
def op_asgn_method
  x += 1
  x
end

# or_asgn should not be flagged
def or_asgn_method
  x ||= 1
  x
end
