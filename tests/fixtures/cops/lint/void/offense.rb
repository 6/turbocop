def foo
  42
  ^^ Lint/Void: Void value expression detected.
  puts 'hello'
end

def bar
  'unused string'
  ^^^^^^^^^^^^^^^ Lint/Void: Void value expression detected.
  do_something
end

def baz
  :symbol
  ^^^^^^^ Lint/Void: Void value expression detected.
  do_work
end

def void_variables
  x = 1
  x
  ^ Lint/Void: Void value expression detected.
  @y = 2
  @y
  ^^ Lint/Void: Void value expression detected.
  @@z = 3
  @@z
  ^^^ Lint/Void: Void value expression detected.
  $global = 4
  $global
  ^^^^^^^ Lint/Void: Void value expression detected.
  "done"
end

def void_constants
  CONST = 1
  CONST
  ^^^^^ Lint/Void: Void value expression detected.
  Foo::BAR
  ^^^^^^^^ Lint/Void: Void value expression detected.
  "done"
end

def void_operators
  a = 1
  b = 2
  a + b
  ^^^^^ Lint/Void: Void value expression detected.
  flag = true
  !flag
  ^^^^^ Lint/Void: Void value expression detected.
  "done"
end

def void_containers
  [1, 2, 3]
  ^^^^^^^^^ Lint/Void: Void value expression detected.
  {a: 1}
  ^^^^^^ Lint/Void: Void value expression detected.
  "done"
end

def void_operator_triple_equals
  a = Object.new
  a === "test"
  ^^^^^^^^^^^^ Lint/Void: Void value expression detected.
  "done"
end

def void_defined
  x = 1
  defined?(x)
  ^^^^^^^^^^^ Lint/Void: Void value expression detected.
  "done"
end

def void_regex
  /pattern/
  ^^^^^^^^^ Lint/Void: Void value expression detected.
  "done"
end

def void_keywords
  __FILE__
  ^^^^^^^^ Lint/Void: Void value expression detected.
  __LINE__
  ^^^^^^^^ Lint/Void: Void value expression detected.
  "done"
end

# Lambda/proc in void context
def void_lambda
  -> { bar }
  ^^^^^^^^^^ Lint/Void: Void value expression detected.
  top
end

def void_lambda_call
  lambda { bar }
  ^^^^^^^^^^^^^^ Lint/Void: Void value expression detected.
  top
end

def void_proc
  proc { bar }
  ^^^^^^^^^^^^ Lint/Void: Void value expression detected.
  top
end

# Literal.freeze in void context
def void_frozen_literal
  'foo'.freeze
  ^^^^^^^^^^^^ Lint/Void: Void value expression detected.
  baz
end

# Void context: initialize — all expressions including last are void
def initialize
  42
  ^^ Lint/Void: Void value expression detected.
  42
  ^^ Lint/Void: Void value expression detected.
end

# Void context: setter method — all expressions including last are void
def foo=(rhs)
  42
  ^^ Lint/Void: Void value expression detected.
  42
  ^^ Lint/Void: Void value expression detected.
end

# Void context: each block — literals are flagged (including last)
array.each do |_item|
  42
  ^^ Lint/Void: Void value expression detected.
  42
  ^^ Lint/Void: Void value expression detected.
end

# Void context: single-expression each block
array.each do |_item|
  42
  ^^ Lint/Void: Void value expression detected.
end

# Void context: tap block
foo.tap do |x|
  42
  ^^ Lint/Void: Void value expression detected.
  42
  ^^ Lint/Void: Void value expression detected.
end

# Void context: for loop — all expressions are void
for _item in array do
  42
  ^^ Lint/Void: Void value expression detected.
  42
  ^^ Lint/Void: Void value expression detected.
end

# Void context: ensure body — all expressions are void
def ensured
  bar
ensure
  42
  ^^ Lint/Void: Void value expression detected.
  42
  ^^ Lint/Void: Void value expression detected.
end

# Void context: single-expression ensure body
def ensured_single
  bar
ensure
  [1, 2, [3]]
  ^^^^^^^^^^^ Lint/Void: Void value expression detected.
end

# Guard clause / modifier conditional — RuboCop unwraps if_type? to check body
def void_guard_clause_var
  x = 5
  x unless condition
  ^ Lint/Void: Void value expression detected.
  top
end

def void_guard_clause_const
  CONST = 5
  CONST unless condition
  ^^^^^ Lint/Void: Void value expression detected.
  top
end

def void_guard_clause_literal
  42 unless condition
  ^^ Lint/Void: Void value expression detected.
  top
end

# Inside unless block (non-last expression)
def void_inside_unless
  CONST = 5
  unless condition
    CONST
    ^^^^^ Lint/Void: Void value expression detected.
  end
  top
end

# Ternary with void expression
def void_ternary
  CONST = 5
  condition ? CONST : nil
              ^^^^^ Lint/Void: Void value expression detected.
  top
end
