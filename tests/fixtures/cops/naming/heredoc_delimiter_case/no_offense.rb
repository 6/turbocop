x = <<~SQL
  SELECT 1
SQL
y = <<-HTML
  <div>
HTML
z = <<TEXT
  hello
TEXT
a = <<~'SQL'
  SELECT 1
SQL
b = <<-"SQL"
  foo
SQL
c = "not a heredoc"
d = 'also not a heredoc'
# Non-word delimiters should not trigger case check
e = <<-'.,.,',
  foo
.,.,
f = <<~'---'
  bar
---
g = <<-'+'
  baz
+
