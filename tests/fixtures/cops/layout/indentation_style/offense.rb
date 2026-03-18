def foo
	bar
^ Layout/IndentationStyle: Tab detected in indentation.
end

def baz
	qux
^ Layout/IndentationStyle: Tab detected in indentation.
	corge
^ Layout/IndentationStyle: Tab detected in indentation.
end

=begin
	Arabic (Windows)	Windows-1256
^ Layout/IndentationStyle: Tab detected in indentation.
	Baltic (Windows)	Windows-1257
^ Layout/IndentationStyle: Tab detected in indentation.
	Hebrew (Windows)	Windows-1255
^ Layout/IndentationStyle: Tab detected in indentation.
=end

# Tab mixed with spaces inside a regex literal should be flagged
KEYWORDS = /( bool       | byte       | complex64
             | complex128 | error      | float32
      	     | float64    | int8       | int16
      ^ Layout/IndentationStyle: Tab detected in indentation.
             )\b/x

# Heredoc closing tag with tab indentation should be flagged
execute <<-SQL
	SELECT * FROM users
	SQL
^ Layout/IndentationStyle: Tab detected in indentation.
