def foo
  bar
end

def baz
  qux
  corge
end

=begin
  Arabic (Windows)	Windows-1256
  Baltic (Windows)	Windows-1257
  Hebrew (Windows)	Windows-1255
=end

# Tab mixed with spaces inside a regex literal should be flagged
KEYWORDS = /( bool       | byte       | complex64
             | complex128 | error      | float32
             | float64    | int8       | int16
             )\b/x

# Heredoc closing tag with tab indentation should be flagged
execute <<-SQL
	SELECT * FROM users
  SQL
