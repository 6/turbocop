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
