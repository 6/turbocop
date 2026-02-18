collection.each { puts _1 }
items.map { _1.to_s }
collection.each do |item|
  puts item
end
items.map do |x|
  x.to_s
end
foo { |a| bar(a) }
