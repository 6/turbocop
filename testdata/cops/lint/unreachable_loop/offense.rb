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

# next in inner loop does NOT prevent outer loop from being flagged
until x > 0
^^^^^^^^^^^ Lint/UnreachableLoop: This loop will have at most one iteration.
  items.each do |item|
    next if item.odd?
    break
  end
  if x > 0
    break
  else
    raise MyError
  end
end

# case-when-else with all branches breaking
while x > 0
^^^^^^^^^^^ Lint/UnreachableLoop: This loop will have at most one iteration.
  case x
  when 1
    break
  else
    raise MyError
  end
end

# if-else with all branches breaking
while x > 0
^^^^^^^^^^^ Lint/UnreachableLoop: This loop will have at most one iteration.
  if condition
    break
  else
    raise MyError
  end
end
