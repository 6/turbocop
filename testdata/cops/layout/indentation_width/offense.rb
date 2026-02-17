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
