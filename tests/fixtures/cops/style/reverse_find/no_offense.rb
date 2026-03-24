array.rfind { |x| x > 0 }
array.find { |x| x > 0 }
array.reverse.map { |x| x * 2 }
array.reverse
x = [1, 2, 3]
y = x.find(&:odd?)

# block_pass with variable (not symbol) — RuboCop does not flag these
arr.reverse.find(&block)
@messages.reverse.find(&block)
ancestors.reverse.detect(&block)
arr.reverse.find(&method(:even?))
