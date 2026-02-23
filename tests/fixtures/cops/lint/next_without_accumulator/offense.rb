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
