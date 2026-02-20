foo(a,
  b)

bar(
  a,
  b
)

baz(a, b, c)

puts("hello")

qux(
  one,
  two,
  three
)

# Method call with do...end block — closing paren on same line as block end
define_method(method, &lambda do |*args, **kwargs|
  nf = nested_field
  nf ? nf.send(method, *args, **kwargs) : super(*args, **kwargs)
end)

# Heredoc argument with method call (<<~SQL.tr) — closing paren after heredoc
query.joins!(<<~SQL.tr("\n", "")
  inner join (
    select stories.id from stories
  ) as t on t.id = stories.id
SQL
            )

# Plain heredoc argument — closing paren after heredoc
foo(<<~RUBY
  some
  content
RUBY
)
