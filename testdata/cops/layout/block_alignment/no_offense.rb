items.each do |x|
  puts x
end

items.each { |x| puts x }

[1, 2].map do |x|
  x * 2
end
