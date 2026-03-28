(1..4).reduce(0) do |acc, el|
  el
  ^^ Lint/UnmodifiedReduceAccumulator: Ensure the accumulator `acc` will be modified by `reduce`.
end
(1..4).inject(0) do |acc, el|
  el
  ^^ Lint/UnmodifiedReduceAccumulator: Ensure the accumulator `acc` will be modified by `inject`.
end
(1..4).reduce do |acc, el|
  el
  ^^ Lint/UnmodifiedReduceAccumulator: Ensure the accumulator `acc` will be modified by `reduce`.
end
%w(a b c).reduce({}) do |acc, letter|
  acc[foo]
  ^^^^^^^^ Lint/UnmodifiedReduceAccumulator: Do not return an element of the accumulator in `reduce`.
end
%w(a b c).inject({}) do |acc, letter|
  acc[foo] = bar
  ^^^^^^^^^^^^^^ Lint/UnmodifiedReduceAccumulator: Do not return an element of the accumulator in `inject`.
end
(1..4).reduce(0) do |acc, el|
  next el if el.even?
       ^^ Lint/UnmodifiedReduceAccumulator: Ensure the accumulator `acc` will be modified by `reduce`.
  acc += 1
end

items.inject(0) do |memo, item|
  expect(item).to eq([1, 2, 3])
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/UnmodifiedReduceAccumulator: Ensure the accumulator `memo` will be modified by `inject`.
end

# nitrocop-expect: 25:31 Lint/UnmodifiedReduceAccumulator: Ensure the accumulator `memo` will be modified by `inject`.
items.inject(0) { |memo, item| expect(item).to eq([1, 2, 3]) }

describe "Enumerable#inject" do
  it "passes all each args to its block" do
    test_enum.inject(0) { |memo, item| expect(item).to eq([1, 2, 3]) }
                                       ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/UnmodifiedReduceAccumulator: Ensure the accumulator `memo` will be modified by `inject`.
  end
end

# FN fix: next element in branch even though accumulator used in last expr
items.reduce(true) do |all_ok, item|
  if condition
    next item
         ^^^^ Lint/UnmodifiedReduceAccumulator: Ensure the accumulator `all_ok` will be modified by `reduce`.
  end
  item.process && all_ok
end

# FN fix: next element when accumulator returned conditionally in other branch
values.reduce(nil) do |memo, value|
  next value if memo.nil?
       ^^^^^ Lint/UnmodifiedReduceAccumulator: Ensure the accumulator `memo` will be modified by `reduce`.
  memo.combine(value)
end

# FN fix: accumulator index with transformed element key
key.split(".").reduce(DEFAULTS) { |defaults, k| defaults[k.to_sym] }
                                                ^^^^^^^^^^^^^^^^^^ Lint/UnmodifiedReduceAccumulator: Do not return an element of the accumulator in `reduce`.

# FN fix: accumulator index returned as last expression in multi-line block
hierarchy.reduce(location_map) do |map, val|
  if val == hierarchy.last
    map[db[val]] ||= []
    map[db[val]] << item
  else
    map[db[val]] ||= {}
  end
  map[db[val]]
  ^^^^^^^^^^^^ Lint/UnmodifiedReduceAccumulator: Do not return an element of the accumulator in `reduce`.
end

# FN fix: bare method call with the element as its only argument
ast.expressions.reduce(DynType) do |t, e|
  type(e)
  ^^^^^^^ Lint/UnmodifiedReduceAccumulator: Ensure the accumulator `t` will be modified by `reduce`.
end

# FN fix: element-only bare call inside an assignment target
children = choice.children.inject('') do |memo, child|
  list_item_for_choice(child)
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/UnmodifiedReduceAccumulator: Ensure the accumulator `memo` will be modified by `inject`.
end

# FN fix: preceding bare call with element interpolation should not count as element mutation
tags.inject(nil) do |prev, tag|
  task("logs/ChangeLog-#{tag}") { |t| changelog[t.name, tag, prev] }
  tag
  ^^^ Lint/UnmodifiedReduceAccumulator: Ensure the accumulator `prev` will be modified by `inject`.
end

# FN fix: same pattern inside an enclosing conditional
unless tags.empty?
  tags.inject(nil) do |prev, tag|
    task("logs/ChangeLog-#{tag}") { |t| changelog[t.name, tag, prev] }
    tag
    ^^^ Lint/UnmodifiedReduceAccumulator: Ensure the accumulator `prev` will be modified by `inject`.
  end
end

# FN fix: bare method call with underscore accumulator
%w[free_ipa posix active_directory].reduce({}) do |_acc, flavor|
  record_flavor_usage(flavor)
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/UnmodifiedReduceAccumulator: Ensure the accumulator `_acc` will be modified by `reduce`.
end

# FN fix: boolean-or fallback still returns an element-only value
registry_set.map { |ext| ext.actions }.flatten.inject({}) do |h, k|
  k[:permitted_attributes] || {}
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/UnmodifiedReduceAccumulator: Ensure the accumulator `h` will be modified by `inject`.
end

# FN fix: boolean-or fallback with underscore accumulator
registry_set.map { |ext| ext.triggers }.flatten.inject([]) do |_, trigger|
  trigger[:permitted_attributes] || []
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/UnmodifiedReduceAccumulator: Ensure the accumulator `_` will be modified by `inject`.
end
