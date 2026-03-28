<<~SQL
  SELECT 1
SQL
.strip
 ^^^^^ Lint/HeredocMethodCallPosition: Put a method call with a HEREDOC receiver on the same line as the HEREDOC opening.

<<~TEXT
  hello
TEXT
.chomp
 ^^^^^ Lint/HeredocMethodCallPosition: Put a method call with a HEREDOC receiver on the same line as the HEREDOC opening.

<<~RUBY
  code
RUBY
.freeze
 ^^^^^^ Lint/HeredocMethodCallPosition: Put a method call with a HEREDOC receiver on the same line as the HEREDOC opening.

SQL[:add] = <<-'SQL' % [
  INSERT OR REPLACE INTO moz_cookies (%s) VALUES (%s)
SQL
  ALL_COLUMNS.join(', '),
  ^^^^^^^^^^^ Lint/HeredocMethodCallPosition: Put a method call with a HEREDOC receiver on the same line as the HEREDOC opening.
  ALL_COLUMNS.map { |col| ":#{col}" }.join(', ')
]
