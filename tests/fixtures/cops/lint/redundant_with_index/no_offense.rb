items.each_with_index { |item, index| puts "#{index}: #{item}" }
items.each { |item| puts item }
items.each_with_index do |item, idx|
  puts "#{idx}: #{item}"
end
items.map.with_index { |item, i| [i, item] }
items.each_with_index { puts "hello" }
items.each_with_index { |*pair| puts pair.inspect }
items.each_with_index { _1; _2 }
items.with_index { |item| puts item }
