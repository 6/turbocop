<<~SQL
^^^^^^ Rails/SquishedSQLHeredocs: Use `<<~SQL.squish` instead of `<<~SQL`.
  SELECT * FROM posts
    WHERE id = 1
SQL

<<-SQL
^^^^^^ Rails/SquishedSQLHeredocs: Use `<<-SQL.squish` instead of `<<-SQL`.
  SELECT * FROM posts;
SQL

execute(<<~SQL, "Post Load")
        ^^^^^^ Rails/SquishedSQLHeredocs: Use `<<~SQL.squish` instead of `<<~SQL`.
  SELECT * FROM posts
    WHERE post_id = 1
SQL
