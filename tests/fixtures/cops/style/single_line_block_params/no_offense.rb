foo.reduce { |acc, elem| acc + elem }
foo.inject { |acc, elem| acc + elem }
foo.map { |x| x * 2 }
foo.each { |item| puts item }
foo.reduce do |a, b|
  a + b
end
x = [1, 2, 3]

# Leading underscores on correct names should be allowed
foo.reduce { |_acc, _elem| 1 }
File.foreach(name).reduce(0) { |acc, _elem| acc + 1 }
items.inject { |_acc, elem| elem }

# Destructuring in block params should be allowed
test.reduce { |acc, (id, _)| acc + id }

# No receiver - should not be flagged
reduce { |x, y| x + y }

# Single param with correct name prefix should be allowed
test.reduce(x) { |acc| foo(acc) }

# Block with no arguments should be allowed
test.reduce { true }

# Keyword params in block should not be flagged (not all args are arg_type?)
Array(replace).inject(source) { |src, from:, to:| src.gsub(from, to) }
