# Properly aligned string concatenation
msg = "foo" \
      "bar"

text = 'hello' \
       'world'

result = "one" \
         "two"

simple = "no continuation"

# Backslash inside heredoc should not be flagged
x = <<~SQL
  SELECT * FROM users \
  WHERE id = 1
SQL

y = <<~SHELL
  echo "hello" \
    "world"
SHELL

# Properly indented in def body (always_indented context)
def some_method
  'x' \
    'y' \
    'z'
end

# Properly indented in block body (always_indented context)
foo do
  "hello" \
    "world"
end

# Aligned inside method call argument (non-indented context)
describe "should not send a notification if a notification service is not" \
         "configured" do
end

# Aligned in case/when branch (when is NOT always-indented parent)
def message
  case style
  when :first_column_after_left_parenthesis
    'Indent the right bracket the same as the first position ' \
    'after the preceding left parenthesis.'
  end
end
