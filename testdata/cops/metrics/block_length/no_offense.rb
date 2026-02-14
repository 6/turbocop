items.each do |item|
  puts item
end

[1, 2, 3].map do |n|
  n * 2
end

things.select do |t|
  t > 0
end

data.each_with_object({}) do |item, hash|
  hash[item] = true
end

results.reject do |r|
  r.nil?
end
