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

# Direct receiver calls on the element are acceptable return values.
@actions.reduce(nil) do |last_date, action|
  action.date
end

# Returning a nested block call rooted in the element is acceptable.
externals.inject(nil) do |o_flag, app_or_hash|
  next if app_or_hash.is_a?(String) || app_or_hash.is_a?(Symbol)
  app_or_hash.inject(nil) do |flag, flag_app_list|
    flag, app_list = flag_app_list
    flag if app_list.include?(app_name)
  end
end

# Returning a block node is acceptable when the return value is not just the element.
appends.inject([]) do |arr, paths|
  Array(paths).each do |path|
    require_asset(path)
  end
end

# A prior call that combines the element with the accumulator makes the bare
# element return acceptable.
scanline_positions.inject do |m, n|
  line_sizes << n - m - 1
  n
end

# Receiver calls nested inside an assignment still count as element modification.
scanline_positions.inject do |pos, delimit|
  scanline = Scanline.new(@filtered_data, pos, (delimit - pos - 1), at)
  delimit
end

# The same applies when the assigned receiver call itself carries an inner block.
scanline_positions.inject do |pos, delimit|
  scanline = Scanline.new(@filtered_data, pos, (delimit - pos - 1), at) do |line|
    line.compress
  end
  delimit
end
