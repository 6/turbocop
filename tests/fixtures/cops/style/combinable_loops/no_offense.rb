# Different collections
items.each { |item| do_something(item) }
other_items.each { |item| do_something(item) }

# Interleaved with code
items.each { |item| foo(item) }
do_something
items.each { |item| bar(item) }

# Different loop methods on same collection
items.reverse_each { |item| do_something(item) }
items.each { |item| do_something(item) }

# Same method with different arguments (e.g., each_slice)
each_slice(2) { |slice| do_something(slice) }
each_slice(3) { |slice| do_something(slice) }

# Different receiver with safe navigation
foo(:bar)&.each { |item| do_something(item) }
foo(:baz)&.each { |item| do_something(item) }

# Empty loops — both bodies empty
items.each {}
items.each {}

# for loops over different collections
for item in items do do_something(item) end
for foo in foos do do_something(foo) end

# for loops interleaved with code
for item in items do do_something(item) end
some_code
for item in items do do_something_else(item, arg) end

# Non-loop methods — map, select, reject are NOT looping methods per RuboCop
items.map { |item| do_something(item) }
items.map { |item| do_something_else(item) }

items.select { |item| item.valid? }
items.select { |item| item.active? }

items.reject { |item| item.nil? }
items.reject { |item| item.empty? }

items.collect { |item| item.to_s }
items.collect { |item| item.to_i }

items.flat_map { |item| item.children }
items.flat_map { |item| item.parents }
