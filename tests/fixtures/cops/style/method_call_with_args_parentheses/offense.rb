# require_parentheses style (default)

# Method calls with receiver and args but no parens
foo.bar 1, 2
^^^^^^^^^^^^ Style/MethodCallWithArgsParentheses: Use parentheses for method calls with arguments.

obj.method "arg"
^^^^^^^^^^^^^^^^ Style/MethodCallWithArgsParentheses: Use parentheses for method calls with arguments.

x.send :message, "data"
^^^^^^^^^^^^^^^^^^^^^^^ Style/MethodCallWithArgsParentheses: Use parentheses for method calls with arguments.

# Receiverless calls inside method defs are NOT macros
def foo
  test a, b
  ^^^^ Style/MethodCallWithArgsParentheses: Use parentheses for method calls with arguments.
end

# Safe navigation operator also flags
top&.test a, b
^^^^^^^^^^^^^^ Style/MethodCallWithArgsParentheses: Use parentheses for method calls with arguments.

# Multiline chained method calls — offense is at start of full expression
expect(described_class.new)
  .to match_array(y)
# nitrocop-expect: 19:0 Style/MethodCallWithArgsParentheses: Use parentheses for method calls with arguments.

custom_fields
  .include? attribute
# nitrocop-expect: 22:0 Style/MethodCallWithArgsParentheses: Use parentheses for method calls with arguments.

# Receiverless call nested as argument to another call in class body
# is NOT a macro (parent in AST is send, not a wrapper)
class MyClass
  foo bar :baz
      ^^^ Style/MethodCallWithArgsParentheses: Use parentheses for method calls with arguments.
end

# Receiverless calls inside case/when in class body are NOT macros
# (case/when are not wrappers in RuboCop's in_macro_scope?)
class MyClass
  case type
  when :foo
    test a, b
    ^^^^ Style/MethodCallWithArgsParentheses: Use parentheses for method calls with arguments.
  end
end

# Receiverless calls inside while/until in class body are NOT macros
class MyClass
  while running
    process_item a
    ^^^^^^^^^^^^ Style/MethodCallWithArgsParentheses: Use parentheses for method calls with arguments.
  end
end

# Receiverless calls inside rescue in class body are NOT macros
# (rescue is not a wrapper in RuboCop's in_macro_scope?)
class MyClass
  begin
    test a, b
    ^^^^ Style/MethodCallWithArgsParentheses: Use parentheses for method calls with arguments.
  rescue
    handle_error a
    ^^^^^^^^^^^^ Style/MethodCallWithArgsParentheses: Use parentheses for method calls with arguments.
  end
end

# Receiverless calls inside ensure in class body are NOT macros
class MyClass
  begin
    test a, b
    ^^^^ Style/MethodCallWithArgsParentheses: Use parentheses for method calls with arguments.
  ensure
    cleanup a
    ^^^^^^^ Style/MethodCallWithArgsParentheses: Use parentheses for method calls with arguments.
  end
end

# yield with args and no parens in method body
def each_item
  yield element
  ^^^^^ Style/MethodCallWithArgsParentheses: Use parentheses for method calls with arguments.
end

# yield with multiple args
def traverse(tree, &block)
  tree.each do |item|
    yield item, tree
    ^^^^^ Style/MethodCallWithArgsParentheses: Use parentheses for method calls with arguments.
  end
end
