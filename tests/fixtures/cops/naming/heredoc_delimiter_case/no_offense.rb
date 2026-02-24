x = <<~SQL
  SELECT 1
SQL
y = <<-HTML
  <div>
HTML
z = <<TEXT
  hello
TEXT
a = <<~'SQL'
  SELECT 1
SQL
b = <<-"SQL"
  foo
SQL
c = "not a heredoc"
d = 'also not a heredoc'
