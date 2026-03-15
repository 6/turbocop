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

# Quoted heredoc tags: <<~'SQL' and <<-'SQL'
execute <<~'SQL'
        ^^^^^^^^ Rails/SquishedSQLHeredocs: Use `<<~'SQL'.squish` instead of `<<~'SQL'`.
  SELECT * FROM records
    WHERE status = 'active'
SQL

create_function :compute, sql_definition: <<-'SQL'
                                          ^^^^^^^^^ Rails/SquishedSQLHeredocs: Use `<<-'SQL'.squish` instead of `<<-'SQL'`.
  SELECT id FROM records
SQL

# Heredoc with /* */ block comments and quoted '---' patterns but no actual -- comments
# Cross-line quote matching must not create phantom -- from '---' inside quotes
execute <<-SQL
        ^^^^^^ Rails/SquishedSQLHeredocs: Use `<<-SQL.squish` instead of `<<-SQL`.
  /* Migrate settings */
  UPDATE users u
  SET
    settings = hstore('key', regexp_replace(s.value, '^--- |\n$', '', 'g'))
  FROM settings s
  WHERE s.thing_id = u.id AND s.thing_type = 'User' AND s.var = 'key';

  /* Migrate second setting */
  UPDATE settings
  SET vals = vals || hstore('path', (
    SELECT regexp_replace(value, '^--- |\n$', '', 'g')
    FROM settings WHERE var = 'path' LIMIT 1
  ))
  WHERE guard = 0;
SQL
