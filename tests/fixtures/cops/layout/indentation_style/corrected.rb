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

# Heredoc closing tag with tab indentation should be flagged
execute <<-SQL
	SELECT * FROM users
  SQL
