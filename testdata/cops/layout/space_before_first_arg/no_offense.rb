foo x
bar 1, 2
baz "hello"
foo(x)
bar(1, 2)
something.method x

# Extra spaces are allowed for alignment (AllowForAlignment: true default)
foo  x
bar  1, 2
baz   "hello"

# Operator methods should not be flagged
2**128
x + 1
a << b
arr[0]
x != y
