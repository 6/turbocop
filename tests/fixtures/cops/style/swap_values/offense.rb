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

tmp = @server
^^^^^^^^^^^^^ Style/SwapValues: Replace this swap with `@server, @server2 = @server2, @server`.
@server = @server2
@server2 = tmp

temp = @index
^^^^^^^^^^^^^ Style/SwapValues: Replace this swap with `@index, @value = @value, @index`.
@index = @value
@value = temp
