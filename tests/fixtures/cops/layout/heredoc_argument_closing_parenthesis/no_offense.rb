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

# Non-heredoc argument after heredoc: paren goes with the trailing arg
foo.bar(<<~TEXT,
    Lots of
    Lovely
    Text
  TEXT
  some_arg: { foo: "bar" }
)

# Heredoc with other param after on same line
foo.bar(<<-SQL, 123)
  foo
SQL

# Heredoc with other param before
foo.bar(123, <<-SQL)
  foo
SQL

# Hash arg with heredoc value
foo.bar(foo: <<-SQL)
  foo
SQL

# Double heredoc correct case
foo.bar(<<-SQL, <<-NOSQL)
  foo
SQL
  bar
NOSQL

# Heredoc inside a block argument
foo(bar do
  baz.quux <<~EOS
  EOS
end)

# Heredoc inside an unless block
foo(unless condition
  bar.baz(<<~EOS)
    text
  EOS
end)

# Method chain on heredoc
foo.bar(
  <<-SQL.tr("z", "t"))
  baz
SQL

# Method invocation on heredoc
foo.bar(
  <<-TEXT.foo)
  foobarbaz
TEXT

# Heredoc with method call on next line
foo.bar(
  <<-TEXT
  foobarbaz
TEXT
  .foo
)

# Double correct case nested
baz(bar(foo.bar(<<-SQL, <<-NOSQL)))
  foo
SQL
  bar
NOSQL

# Double correct case new line
foo.bar(
  <<-SQL, <<-NOSQL)
  foo
SQL
  bar
NOSQL

# Method chain with heredoc argument
do_something(
  Model
    .foo.bar(<<~CODE)
      code
    CODE
    .baz(<<~CODE))
      code
    CODE

# Nested heredoc in parenthesized block argument
foo(unless condition
  bar(baz.quux(<<~EOS))
    text
  EOS
end)

# Proc with heredoc
outer_method(-> {
  inner_method.chain(<<~CODE)
    code
  CODE
})

# Multiple non-heredoc args after heredoc
foo(<<~SQL, 123, 456)
  query
SQL

# Hash arg after heredoc on separate line
foo(<<~SQL,
  query
SQL
  opt: true
)

# Heredoc with trailing comma and non-heredoc arg on next line
bar(<<~TEXT,
  content
TEXT
  key: "value",
  another: 123
)

# Nested call where inner call has heredoc and outer has other args
outer(
  inner(<<~SQL),
    query
  SQL
  other_arg
)

# Heredoc call inside a method definition (ancestor has `end` keyword)
def my_method
  expect_eq(
    result,
    <<~MSG
      hello world
    MSG
  )
end

# Heredoc call inside a class definition
class MyClass
  validate(<<~SQL
    SELECT 1
  SQL
  )
end

# Heredoc call inside a do..end block
items.each do |item|
  process(<<~TEXT
    content here
  TEXT
  )
end

# Heredoc call inside an if block
if condition
  render(<<~HTML
    <div>hello</div>
  HTML
  )
end

# Heredoc call inside nested end-bearing constructs
class Config
  def setup
    validate(<<~YAML
      key: value
    YAML
    )
  end
end
