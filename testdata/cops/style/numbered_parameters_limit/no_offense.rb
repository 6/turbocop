foo { do_something(_1) }
foo { |a, b, c| do_something(a, b, c) }
bar { _1.to_s }
baz { |x| x + 1 }
items.map { _1 * 2 }
collection.each { puts _1 }
