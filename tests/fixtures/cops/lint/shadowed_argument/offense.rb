def foo(bar)
  bar = 'something'
  ^^^^^^^^^^^^^^^^^ Lint/ShadowedArgument: Argument `bar` was shadowed by a local variable before it was used.
  bar
end

def baz(x, y)
  x = 42
  ^^^^^^ Lint/ShadowedArgument: Argument `x` was shadowed by a local variable before it was used.
  x + y
end

def qux(name)
  name = compute_name
  ^^^^^^^^^^^^^^^^^^^^ Lint/ShadowedArgument: Argument `name` was shadowed by a local variable before it was used.
  name
end
