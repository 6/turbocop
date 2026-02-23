collection.each { puts _1 }
items.map { _1.to_s }
collection.each do |item|
  puts item
end
items.map do |x|
  x.to_s
end
foo { |a| bar(a) }
# _1 in a comment should not trigger
items.map { |x| x._1_method }
foo do |item|
  # variable named _1_foo is not a numbered param
  _1_foo = item.name
  puts _1_foo
end
bar { |x| x.to_s + "_1" }
