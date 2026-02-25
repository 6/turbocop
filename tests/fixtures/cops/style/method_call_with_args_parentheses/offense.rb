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
# turbocop-expect: 19:0 Style/MethodCallWithArgsParentheses: Use parentheses for method calls with arguments.

custom_fields
  .include? attribute
# turbocop-expect: 22:0 Style/MethodCallWithArgsParentheses: Use parentheses for method calls with arguments.
