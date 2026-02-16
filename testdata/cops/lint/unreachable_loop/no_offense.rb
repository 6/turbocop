while node
  do_something(node)
  node = node.parent
end

items.each do |item|
  if something?(item)
    return item
  end
end

loop do
  break if done?
  do_something
end
