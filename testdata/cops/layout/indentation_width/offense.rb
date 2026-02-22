class Foo
    x = 1
    ^^^ Layout/IndentationWidth: Use 2 (not 4) spaces for indentation.
end

def bar
 y = 2
 ^^^ Layout/IndentationWidth: Use 2 (not 1) spaces for indentation.
end

if true
      z = 3
      ^^^ Layout/IndentationWidth: Use 2 (not 6) spaces for indentation.
end

items.each do |item|
      process(item)
      ^^^ Layout/IndentationWidth: Use 2 (not 6) spaces for indentation.
end

case x
when 1
      do_something
      ^^^ Layout/IndentationWidth: Use 2 (not 6) spaces for indentation.
end

# Block on chained method — dot on new line, body should indent from dot
source.passive_relationships
      .where(account: Account.local)
      .in_batches do |follows|
  process(follows)
  ^^^ Layout/IndentationWidth: Use 2 (not -4) spaces for indentation.
end

# Another chained block — body indented from end, not dot
Post.includes(:comments)
  .where("stuff")
  .references(:comments)
  .scoping do
  posts = authors(:david)
  ^^^ Layout/IndentationWidth: Use 2 (not 0) spaces for indentation.
end

# Block on chained method — body wrong relative to both dot and end columns
source
  .in_batches do |batch|
      process(batch)
      ^^^ Layout/IndentationWidth: Use 2 (not 4) spaces for indentation.
end

# begin...end block with wrong indentation
begin
x = 1
^^^ Layout/IndentationWidth: Use 2 (not 0) spaces for indentation.
rescue => e
  puts e
end

begin
      require 'builder'
      ^^^ Layout/IndentationWidth: Use 2 (not 6) spaces for indentation.
end

begin
    do_something
    ^^^ Layout/IndentationWidth: Use 2 (not 4) spaces for indentation.
rescue StandardError
  handle
end

# Assignment context: body should be indented from `if` keyword, not `end`
result = if condition
  value_one
  ^^^ Layout/IndentationWidth: Use 2 (not -7) spaces for indentation.
else
  value_two
end

      stream = if scheduler
        Stream.new(scheduler)
        ^^^ Layout/IndentationWidth: Use 2 (not -7) spaces for indentation.
      else
        nil
      end
