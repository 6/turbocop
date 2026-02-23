a = 1
b = 2
tmp = a
^^^^^^^ Style/SwapValues: Replace this swap with `a, b = b, a`.
a = b
b = tmp

x = 10
y = 20
temp = x
^^^^^^^^ Style/SwapValues: Replace this swap with `x, y = y, x`.
x = y
y = temp

foo = :one
bar = :two
t = foo
^^^^^^^ Style/SwapValues: Replace this swap with `foo, bar = bar, foo`.
foo = bar
bar = t
