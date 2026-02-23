def some_method(used, unused)
                      ^^^^^^ Lint/UnusedMethodArgument: Unused method argument - `unused`.
  puts used
end

def foo(bar, baz)
             ^^^ Lint/UnusedMethodArgument: Unused method argument - `baz`.
  bar
end

def calculate(x, y, z)
                    ^ Lint/UnusedMethodArgument: Unused method argument - `z`.
  x + y
end

def protect(*args)
             ^^^^ Lint/UnusedMethodArgument: Unused method argument - `args`.
  do_something
end
