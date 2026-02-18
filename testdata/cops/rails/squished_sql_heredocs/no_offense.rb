<<-SQL.squish
  SELECT * FROM posts
    WHERE id = 1
SQL

<<~SQL.squish
  SELECT * FROM posts
    WHERE id = 1
SQL

<<-SQL
  SELECT * FROM posts
  -- This is a comment, so squish can't be used
    WHERE id = 1
SQL

execute(<<~SQL.squish, "Post Load")
  SELECT * FROM posts
    WHERE post_id = 1
SQL
