def foo x, y
        ^^^^ Style/MethodDefParentheses: Use `def` with parentheses when there are parameters.
  x + y
end

def bar x
        ^ Style/MethodDefParentheses: Use `def` with parentheses when there are parameters.
  x
end

def baz a, b, c
        ^^^^^^^ Style/MethodDefParentheses: Use `def` with parentheses when there are parameters.
  a + b + c
end
