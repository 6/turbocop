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

# Accumulator returned via next in another branch (FP fix)
types.inject do |type1, type2|
  next type2 if type1.is_a?(Foo)
  next type1 if type2.is_a?(Foo)
  type1
end

# next with accumulator makes element return acceptable
values.reduce(nil) do |result, value|
  next value if something?
  result
end

# Returning accumulator index with element key is acceptable
foo.reduce { |result, key| result[key] }

processors.inject([request, headers]) do |packet, processor|
  processor.call(*packet)
end

scopes.reverse_each.reduce(compiled) do |body, scope|
  scope.wrap(body: [body])
end

# Bare calls only count as element modification when the element is passed alongside
# at least one other argument, matching RuboCop's `method(el, ...)` behavior.
values.reduce do |acc, el|
  method(el, 1)
  el
end

# Boolean fallbacks still accept method-chain returns that go beyond the bare element.
entities.reduce(0) do |index, entity|
  entity[:indices].last || []
end

# A fallback method call adds a non-element expression value, so it stays acceptable.
registry_set.map { |ext| ext.actions }.flatten.inject({}) do |h, k|
  k[:permitted_attributes] || fallback
end
