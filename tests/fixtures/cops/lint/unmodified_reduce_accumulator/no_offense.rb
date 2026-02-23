(1..4).reduce(0) do |acc, el|
  acc + el
end
(1..4).reduce(0) do |acc, el|
  acc
end
(1..4).reduce(0) do |acc, el|
  acc += el
end
(1..4).reduce(0) do |acc, el|
  acc << el
end
values.reduce(:+)
values.reduce do
  do_something
end
foo.reduce { |result, key| result.method(key) }

# Method chains on the element are acceptable (not just the bare element)
entities.reduce(0) do |index, entity|
  entity[:indices].last
end

# Accumulator returned via break inside conditional
parent.each_child_node.inject(false) do |if_type, child|
  break if_type if condition
  child.if_type?
end
