arr.sort_by(&:name)
arr.sort
arr.sort { |a, b| a <=> b }
array.sort_by { |a| a.baz }
array.sort { |a, b| a.foo <=> b.bar }
# safe navigation (&.) should not be flagged
items&.sort { |a, b| a.name <=> b.name }
# method calls with arguments should not be flagged
items.sort { |a, b| a.fetch(:key) <=> b.fetch(:key) }
