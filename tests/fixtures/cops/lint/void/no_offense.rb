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
x = [1, 2, 3]

# Assigned if statement — not void
x = if condition
      42
    end
do_something

# Assigned ternary — not void
x = condition ? 42 : nil
do_something

# Assigned modifier if — not void
x = (42 if condition)
do_something

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

# Void operators exempted inside each blocks (enumerator filter pattern)
enumerator_as_filter.each do |item|
  item == 42
end

# Multi-statement each block — operator on last line is exempt
enumerator_as_filter.each do |item|
  puts item
  item == 42
end

# Lambda/proc with .call — not void (has side effects)
def not_void_lambda_call
  -> { bar }.call
  top
end

def not_void_proc_call
  lambda { bar }.call
  top
end

# Frozen non-literal — not entirely literal
def frozen_non_literal
  foo.freeze
  baz
end

# Operator with dot notation and no args — not flagged
def dot_operator_no_args
  a.+
  something
end

def safe_nav_operator_no_args
  a&.+
  something
end

# if without body — not void
if some_condition
end
puts :ok

# method call inside modifier unless — has side effects, not void
def guard_clause_with_side_effects
  do_something unless condition
  top
end

# return with guard clause — not void
def return_guard
  return 42 unless condition
  top
end

# Empty numblock each — not void
array.each { }

# Short lambda call — not void
lambda.(a)
top
