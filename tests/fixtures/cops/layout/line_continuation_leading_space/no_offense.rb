x = 'this text is too ' \
    'long'

y = 'this text contains a lot of               ' \
    'spaces'

z = "another example " \
    "without leading space"

a = "single line string"
b = 'no continuation'

# Backslash inside heredoc should not be flagged
x = <<~SQL
  SELECT * FROM users \
  WHERE id = 1
SQL

y = <<~SHELL
  echo hello \
  world
SHELL
