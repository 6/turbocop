if condition
  do_something
elsif other
  do_other
end

case x
when 1
  :foo
when 2
  :bar
when 3
  :baz
end

# Heredocs with different content in case/when branches (not duplicates)
case style
when :a
  <<~RUBY
    hello world
  RUBY
when :b
  <<~RUBY
    goodbye world
  RUBY
end

# Heredocs with different content in if/else branches (not duplicates)
if condition
  expect_offense(<<~RUBY)
    x = 1
    ^^^ Error one.
  RUBY
else
  expect_offense(<<~RUBY)
    x = 1
    ^^^ Error two.
  RUBY
end
