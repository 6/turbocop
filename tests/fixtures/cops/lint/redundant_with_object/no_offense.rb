items.each_with_object([]) { |item, acc| acc << item }
items.each { |item| puts item }
items.each_with_object({}) do |item, hash|
  hash[item] = true
end
items.inject({}) { |acc, item| acc.merge(item => true) }
