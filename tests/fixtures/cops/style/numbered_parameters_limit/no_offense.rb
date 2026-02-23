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
# Using only _2 (1 unique param, not 2) — should not fire with Max: 1
attributes.map { Condition.new(_2) }
hash.each { use_only_hash_value(_2) }
