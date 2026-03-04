[1, 2, 3].sum
[1, 2, 3].inject(:*)
[1, 2, 3].reduce(1, :*)
arr.sum { |x| x.value }
array.inject { |acc, elem| elem * 2 }
array.reduce(0) { |acc, elem| acc * elem }
array.inject(0) { |acc, elem| acc - elem }
array.inject(0) { |sum| sum + 1 }
items.map(&:count).sum { |x| x ** 2 }
items.map(&:count).sum(&:count)
