one_plus_one = 1 \
  + 1

two_plus_two = 2 \
  + 2

three = 3 \
  + 0

# Backslash inside heredoc should not be flagged
x = <<~SQL
  SELECT * FROM users \
  WHERE id = 1
SQL

y = <<~SHELL
  echo hello \
  world
SHELL

z = <<~RUBY
  foo(bar, \
      baz)
RUBY
