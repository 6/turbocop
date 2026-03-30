x = <<-RUBY
something
^^^^^^^^^ Layout/HeredocIndentation: Use 2 spaces for indentation in a heredoc by using `<<~` instead of `<<-`.
RUBY

y = <<-TEXT
hello world
^^^^^^^^^^^ Layout/HeredocIndentation: Use 2 spaces for indentation in a heredoc by using `<<~` instead of `<<-`.
TEXT

z = <<-SQL
SELECT * FROM users
^^^^^^^^^^^^^^^^^^^ Layout/HeredocIndentation: Use 2 spaces for indentation in a heredoc by using `<<~` instead of `<<-`.
SQL

# <<- with .squish and indented body should be flagged
execute <<-SQL.squish
  INSERT INTO accounts (name) VALUES ('test')
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Layout/HeredocIndentation: Use 2 spaces for indentation in a heredoc by using `<<~` instead of `<<-`.
SQL

result = ActiveRecord::Base.connection.exec_insert(<<-SQL.squish)
    SELECT id, name
^^^^^^^^^^^^^^^^^^^^ Layout/HeredocIndentation: Use 2 spaces for indentation in a heredoc by using `<<~` instead of `<<-`.
    FROM users
    WHERE id = 1
SQL

Status.find_by_sql(<<-SQL.squish)
      WITH RECURSIVE search_tree(id, path) AS (
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Layout/HeredocIndentation: Use 2 spaces for indentation in a heredoc by using `<<~` instead of `<<-`.
        SELECT id, ARRAY[id] FROM statuses WHERE id = :id
      )
      SELECT id FROM search_tree
SQL

# <<- with .squish on separate line after closing delimiter
query = <<-SQL
  SELECT MAX(title)
^^^^^^^^^^^^^^^^^^^^^^^^^^ Layout/HeredocIndentation: Use 2 spaces for indentation in a heredoc by using `<<~` instead of `<<-`.
    FROM articles
SQL
.squish

join = <<-SQL
  LEFT OUTER JOIN (
^^^^^^^^^^^^^^^^^^^^^^ Layout/HeredocIndentation: Use 2 spaces for indentation in a heredoc by using `<<~` instead of `<<-`.
      SELECT comments.*
    FROM comments
  ) AS latest_comment
SQL
.squish

# <<- with .squish! on separate line after closing delimiter
value = <<-TEXT
  some content here
^^^^^^^^^^^^^^^^^^^^^^ Layout/HeredocIndentation: Use 2 spaces for indentation in a heredoc by using `<<~` instead of `<<-`.
TEXT
.squish!

# Bare <<WORD heredocs with body at column 0 should be flagged
a = <<RUBY
something
^^^^^^^^^ Layout/HeredocIndentation: Use 2 spaces for indentation in a heredoc by using `<<~` instead of `<<`.
RUBY

b = <<TEXT
hello world
^^^^^^^^^^^ Layout/HeredocIndentation: Use 2 spaces for indentation in a heredoc by using `<<~` instead of `<<`.
TEXT

c = <<SQL
SELECT * FROM users
^^^^^^^^^^^^^^^^^^^ Layout/HeredocIndentation: Use 2 spaces for indentation in a heredoc by using `<<~` instead of `<<`.
SQL

# <<~ with tab-indented opening line still expects two extra indentation characters
test do
	assert_equal error.message, <<~ERROR
  Type mismatch
^^^^^^^^^^^^^^^ Layout/HeredocIndentation: Use 2 spaces for indentation in a heredoc.
	ERROR
end
