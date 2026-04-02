foo + bar
foo - 42
foo == bar
a * b
x << y
z >> w

# Chained parenthesized operator call — removing dot changes semantics
scopes.-(%i[show_dashboard]).any?
foo.+(@bar).to_s

# Constant receiver — RuboCop skips when receiver is const_type?
Tree.<<(result)
Image.>> dest
Foo.+ bar
Foo.-(baz)
Array.&(other)
Hash.== other

# Parenthesized operator call nested inside another method call
# RuboCop only skips when the RHS has a Parser-style truthy first child
expect([one].==([two])).to eq(true)

# Parenthesized operator call as last arg of non-parenthesized method call
# The `,` before receiver indicates nesting even when `)` is at end of line.
# Bare no-receiver RHS cases stay offensive; see offense.rb.
assert_equal 1, c.<=>(@item)
be_close(6543.21.%(137), tolerance)

# Parenthesized operator call as sole space-separated arg (no comma before receiver)
# Bare no-receiver RHS cases stay offensive; see offense.rb.
assert_nil @c2.<=>(Gem.loaded_specs.values.first)

# Parenthesized operator call as RHS of another operator
# Grandparent is == send node, so RuboCop skips
result.should == value.%(0xffffffff)

# Splat, block_pass, kwsplat arguments — removing dot would be invalid syntax
foo.+(*args)
foo.-(**kwargs)
foo.==(&blk)
