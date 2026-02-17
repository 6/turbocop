x = <<~RUBY
  something
RUBY

y = <<~TEXT
  hello world
TEXT

z = <<~SQL
  SELECT * FROM users
SQL

a = <<-RUBY
  indented body is fine
RUBY

b = <<PLAIN
no indentation expected
PLAIN

# <<~ with correct indentation (2 spaces from base)
def method_body
  <<~SQL
    SELECT * FROM users
  SQL
end

# <<~ at top-level with 2-space indent body
c = <<~HEREDOC
  line one
  line two
HEREDOC

# Empty heredocs are fine
d = <<~RUBY
RUBY
