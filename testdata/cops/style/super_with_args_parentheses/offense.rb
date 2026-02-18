def foo
  super a, b
  ^^^^^^^^^^ Style/SuperWithArgsParentheses: Use parentheses for `super` with arguments.
end

def bar
  super x
  ^^^^^^^ Style/SuperWithArgsParentheses: Use parentheses for `super` with arguments.
end

def baz
  super 1, 2, 3
  ^^^^^^^^^^^^^ Style/SuperWithArgsParentheses: Use parentheses for `super` with arguments.
end
