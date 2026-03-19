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
