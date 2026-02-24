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

# Mutation operators are NOT void (they have side effects)
def mutation_operators
  lines = []
  lines << "hello"
  lines << "world"
  code = ""
  code << generate_content
  @items << item
  result = []
  result << self
  puts result
end

# Bitwise operators on variables are NOT void
def bitwise_ops
  flags = 0
  flags | FLAG_A
  flags & MASK
  flags ^ toggle
  value >> 2
  "done"
end

# Arrays/hashes with non-literal elements are NOT void
def non_literal_containers
  [foo, bar, baz]
  {name: @user.name, email: current_user.email}
  [1, method_call, 3]
  {key: some_variable}
  "done"
end

# Ranges are not void (RuboCop excludes them)
def range_usage
  1..10
  'a'..'z'
  "done"
end

# Interpolated strings may have side effects
def interpolated
  "#{expensive_computation}"
  "done"
end
