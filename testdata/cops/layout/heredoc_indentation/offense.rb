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
