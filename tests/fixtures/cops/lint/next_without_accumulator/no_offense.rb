result = (1..4).reduce(0) do |acc, i|
  next acc if i.odd?
  acc + i
end

result = (1..4).inject(0) do |acc, i|
  next acc if i.odd?
  acc + i
end

result = keys.reduce(raw) do |memo, key|
  next memo unless memo
  memo[key]
end

result = constants.inject({}) do |memo, name|
  value = const_get(name)
  next memo unless Integer === value
  memo[name] = value
  memo
end
