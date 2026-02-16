foo(<<~SQL
  SELECT * FROM t
SQL
)
^ Layout/HeredocArgumentClosingParenthesis: Put the closing parenthesis for a method call with a HEREDOC parameter on the same line as the HEREDOC opening.

bar(<<~RUBY
  puts 'hello'
RUBY
)
^ Layout/HeredocArgumentClosingParenthesis: Put the closing parenthesis for a method call with a HEREDOC parameter on the same line as the HEREDOC opening.

baz(<<~TEXT
  Some text here
TEXT
)
^ Layout/HeredocArgumentClosingParenthesis: Put the closing parenthesis for a method call with a HEREDOC parameter on the same line as the HEREDOC opening.
