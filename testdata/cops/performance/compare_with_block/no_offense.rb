arr.sort_by(&:name)
arr.sort
arr.sort { |a, b| a <=> b }
array.sort_by { |a| a.baz }
array.sort { |a, b| a.foo <=> b.bar }
