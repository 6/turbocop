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

# Implicit-self swap: RuboCop detects cycle via add_self_to_getters
self.issue_to, self.issue_from = issue_from, issue_to
self.left_child, self.right_child = right_child, left_child

# Nested groups with splats — flattened count mismatches RHS
(a, *b), c, (*d, (e, *f, g)) = 1, 2, 3, 4
(*a, b), c = [[1, 2, 3], 4]
