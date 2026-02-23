x.any?
x.none?
x.count
x.count > 1
x.count == 5
x.empty?
# length and size are NOT checked (String implements them too)
x.length.positive?
x.size.positive?
x.length.zero?
x.size > 0
x.size == 0
# count with positional argument is excluded
x.count(:foo).positive?
x.count(1) > 0
