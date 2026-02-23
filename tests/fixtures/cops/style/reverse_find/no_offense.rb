array.rfind { |x| x > 0 }
array.find { |x| x > 0 }
array.reverse.map { |x| x * 2 }
array.reverse
x = [1, 2, 3]
y = x.find(&:odd?)
