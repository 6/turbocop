def foo
  bar
end

class Baz
  def qux
    corge
  end
end

x = 1
y = 2

# Tabs inside heredocs should not be flagged
sql = <<-SQL
	SELECT *
	FROM users
SQL

msg = <<~HEREDOC
	indented with tab
	also with tab
HEREDOC

content = <<-CONTENT
	tab indented content
CONTENT
