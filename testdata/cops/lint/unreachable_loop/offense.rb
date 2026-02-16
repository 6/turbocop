while node
^^^^^ Lint/UnreachableLoop: This loop will have at most one iteration.
  do_something(node)
  node = node.parent
  break
end

items.each do |item|
^^^^^^^^^^ Lint/UnreachableLoop: This loop will have at most one iteration.
  return item if something?(item)
  raise NotFoundError
end

loop do
^^^^ Lint/UnreachableLoop: This loop will have at most one iteration.
  do_something
  break
end
