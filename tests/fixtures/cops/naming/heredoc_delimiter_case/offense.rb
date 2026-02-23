x = <<~sql
       ^^^ Naming/HeredocDelimiterCase: Use uppercase heredoc delimiters.
  SELECT 1
sql
y = <<~html
       ^^^^ Naming/HeredocDelimiterCase: Use uppercase heredoc delimiters.
  <div>
html
z = <<~text
       ^^^^ Naming/HeredocDelimiterCase: Use uppercase heredoc delimiters.
  hello
text
