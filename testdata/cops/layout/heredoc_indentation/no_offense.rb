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
