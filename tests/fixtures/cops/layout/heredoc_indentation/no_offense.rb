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

# Interpolated squiggly heredoc with correct indentation
def generate_response
  <<~RESPONSE
    #{total_count > 100 ? "First #{total_count}" : "All #{total_count}"}
    #{articles.map(&:to_text).join("\n")}
  RESPONSE
end

# Multiple heredocs on the same line
def test_multiple
  method_call <<~FIRST, <<~SECOND
    first body
    more first
  FIRST
    second body
    more second
  SECOND
end
