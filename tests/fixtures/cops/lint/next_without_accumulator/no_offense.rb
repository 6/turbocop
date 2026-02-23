result = (1..4).reduce(0) do |acc, i|
  next acc if i.odd?
  acc + i
end

result = (1..4).inject(0) do |acc, i|
  next acc if i.odd?
  acc + i
end
