a = 1
b = 2
c = 3
a, b = foo
a, *b = [1, 2, 3]
x, y = array

# Swap idioms should not be flagged
a, b = b, a
x, y = y, x
min, max = max, min

# Indexed swaps
array[0], array[1] = array[1], array[0]
@state[i], @state[j] = @state[j], @state[i]
self[0], self[2] = self[2], self[0]

# Method call swaps
node.left, node.right = node.right, node.left

# Conditional swap
min_x, max_x = max_x, min_x if min_x > max_x

# Expression-based cycles (Fibonacci pattern, etc.)
x, y = y, x + y
a, b = (a + b), (a - b)

# Nested group with size mismatch (RuboCop doesn't flag)
(a, b), c = [1, 2], 3
