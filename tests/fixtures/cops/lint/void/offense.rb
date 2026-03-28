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
    ^ Lint/Void: Void value expression detected.
  flag = true
  !flag
  ^ Lint/Void: Void value expression detected.
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
    ^^^ Lint/Void: Void value expression detected.
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

# Interpolated strings are void literals (RuboCop considers dstr as literal)
def void_interpolated_string
  "#{1+1} #{1+1} #{1+1}"
  ^^^^^^^^^^^^^^^^^^^^^^^ Lint/Void: Void value expression detected.
  do_something
end

# Multiline operator — report at operator position, not expression start
def void_multiline_operator
  a.foo(
    bar
  ).should == true
           ^^ Lint/Void: Void value expression detected.
  something_else
end

# Void context: []= is a setter method (assignment_method? in RuboCop)
def []=(key, value)
  @hash ||= {}
  @hash[key] = value
  value
  ^^^^^ Lint/Void: Void value expression detected.
end

# Void context: singleton setter method (def self.foo=)
def self.log_output=(output)
  @log_output = output
  @logger = new_logger(output)
  output
  ^^^^^^ Lint/Void: Void value expression detected.
end

# Operator inside nested block within each — should NOT be exempted
# RuboCop only exempts operators in the direct each block body
[1, 2].each do |item|
  it "test #{item}" do
    result.should == item
                  ^^ Lint/Void: Void value expression detected.
    other_result
  end
end

# Hash with splat in void context
def void_hash_splat
  { **x }
  ^^^^^^ Lint/Void: Void value expression detected.
  do_something
end

# Hash with interpolated string key in void context
def void_hash_interp_key
  { "foo #{ 2 }": 1 }
  ^^^^^^^^^^^^^^^^^^^^ Lint/Void: Void value expression detected.
  do_something
end

def void_hash_interp_key2
  { "foo #{2}": 1 }
  ^^^^^^^^^^^^^^^^^^ Lint/Void: Void value expression detected.
  do_something
end

# Interpolated word array in void context
def void_interp_word_array
  %W(one two #{1})
  ^^^^^^^^^^^^^^^^ Lint/Void: Void value expression detected.
  do_something
end

def void_interp_word_array2
  %W( one  two #{ 1 } )
  ^^^^^^^^^^^^^^^^^^^^^^ Lint/Void: Void value expression detected.
  do_something
end

def void_interp_word_array3
  %W(#{1}2)
  ^^^^^^^^^ Lint/Void: Void value expression detected.
  do_something
end

# Interpolated symbol array in void context
def void_interp_symbol_array
  %I( one  two #{ 1 } )
  ^^^^^^^^^^^^^^^^^^^^^^ Lint/Void: Void value expression detected.
  do_something
end

# Hashes with range values are still entirely literal in RuboCop
def void_hash_range_values
  { foo: ...bar }
  ^^^^^^^^^^^^^^^ Lint/Void: Void value expression detected.
  { foo: ..bar }
  ^^^^^^^^^^^^^^ Lint/Void: Void value expression detected.
  top
end
