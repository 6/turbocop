items.each { |x| puts x }

items.map do |x|
  x * 2
end

[1, 2].each { |i| i + 1 }

items.select do |x|
  x > 0
end

3.times { |i| puts i }
