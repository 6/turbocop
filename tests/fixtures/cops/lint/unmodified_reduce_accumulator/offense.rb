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
