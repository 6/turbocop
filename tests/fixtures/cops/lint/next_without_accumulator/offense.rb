result = (1..4).reduce(0) do |acc, i|
  next if i.odd?
  ^^^^ Lint/NextWithoutAccumulator: Use `next` with an accumulator argument in a `reduce`.
  acc + i
end

result = (1..4).inject(0) do |acc, i|
  next if i.odd?
  ^^^^ Lint/NextWithoutAccumulator: Use `next` with an accumulator argument in a `reduce`.
  acc + i
end

result = items.reduce([]) do |acc, item|
  next if item.nil?
  ^^^^ Lint/NextWithoutAccumulator: Use `next` with an accumulator argument in a `reduce`.
  acc << item
end

result = keys.reduce(raw) do |memo, key|
  next unless memo
  ^^^^ Lint/NextWithoutAccumulator: Use `next` with an accumulator argument in a `reduce`.
  memo[key]
end

result = constants.inject({}) do |memo, name|
  value = const_get(name)
  next unless Integer === value
  ^^^^ Lint/NextWithoutAccumulator: Use `next` with an accumulator argument in a `reduce`.
  memo[name] = value
  memo
end
