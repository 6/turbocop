x = <<~sql
  SELECT 1
sql
^^^ Naming/HeredocDelimiterCase: Use uppercase heredoc delimiters.
y = <<-html
  <div>
html
^^^^ Naming/HeredocDelimiterCase: Use uppercase heredoc delimiters.
z = <<text
  hello
text
^^^^ Naming/HeredocDelimiterCase: Use uppercase heredoc delimiters.
a = <<~'sql'
  SELECT 1
sql
^^^ Naming/HeredocDelimiterCase: Use uppercase heredoc delimiters.
b = <<-"Sql"
  foo
Sql
^^^ Naming/HeredocDelimiterCase: Use uppercase heredoc delimiters.
