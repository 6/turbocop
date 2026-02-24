# Return values (last expression in method)
def returns_literal
  42
end

def returns_var
  x = 1
  x
end

def returns_constant
  CONST
end

# Method calls have side effects — not void
def side_effects
  puts "hello"
  save!
  "done"
end

# Assignments are not void
def assignments
  x = 1
  y = x + 2
  y
end

# Single expression method body
def single_expr
  "hello"
end

# Conditional expressions
x = 'hello'
puts x
result = :symbol
42 if condition
x = [1, 2, 3]
