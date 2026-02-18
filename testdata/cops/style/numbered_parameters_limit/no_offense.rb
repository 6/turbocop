foo { do_something(_1) }
foo { |a, b, c| do_something(a, b, c) }
bar { _1.to_s }
baz { |x| x + 1 }
items.map { _1 * 2 }
collection.each { puts _1 }
# _1 and _2 in a comment should not trigger
items.map { |x| x.to_s + "_1_2" }
foo do |item|
  _1_var = item.name
  _2_var = item.value
  puts _1_var + _2_var
end
