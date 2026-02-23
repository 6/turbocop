foo(<<~SQL)
  SELECT * FROM t
SQL

bar(<<~RUBY)
  puts 'hello'
RUBY

baz("hello", "world")

qux(
  first,
  second
)
