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
  1..10
  ^^^^^ Lint/Void: Void value expression detected.
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
