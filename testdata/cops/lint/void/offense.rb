def foo
  42
  ^ Lint/Void: Void value expression detected.
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
