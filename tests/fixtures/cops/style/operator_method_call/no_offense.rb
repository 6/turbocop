foo + bar
foo - 42
foo == bar
a * b
x << y
z >> w

# Chained parenthesized operator call — removing dot changes semantics
scopes.-(%i[show_dashboard]).any?
array.-(other).length

# Constant receiver — RuboCop skips when receiver is const_type?
Tree.<<(result)
Image.>> dest
Foo.+ bar
Foo.-(baz)
Array.&(other)
Hash.== other

# Parenthesized operator call nested inside another method call
# RuboCop skips when the operator call is parenthesized and its parent is a send node
expect(one.==(two)).to eq(true)
expect([one].==([two])).to eq(true)
assert(a.+(b))
foo(x.-(y))
bar(x.*(y), z)

# Splat, block_pass, kwsplat arguments — removing dot would be invalid syntax
foo.+(*args)
foo.-(**kwargs)
foo.==(&blk)
