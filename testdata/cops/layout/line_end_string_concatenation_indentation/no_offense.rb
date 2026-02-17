text = 'hello' \
'world'

msg = "foo" \
"bar"

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
