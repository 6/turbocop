x = y
@foo = @bar
$baz = $qux
y = x + 1
name = other_name
@@count = @@total

# Compound assignment with different values
foo ||= bar
foo &&= bar
@foo ||= compute_value
@@cls &&= other

# Multi-write with swapped/different values
foo, bar = bar, foo
foo, bar = *something
foo, bar = something

# Attribute assignment with different attributes or receivers
foo.bar = foo.baz
bar.foo = baz.foo
foo.bar = foo.bar + 1
foo.bar = foo.bar(arg)
foo.bar = true

# Index assignment with different keys or receivers
foo["bar"] = foo["baz"]
bar["foo"] = baz["foo"]
foo["bar"] = foo["bar"] + 1
foo[1] = foo[2]
foo[1.2] = foo[2.2]
foo[FOO] = foo[BAR]
foo[:foo] = foo[:bar]
foo[@var1] = foo[@var2]
foo[@@var1] = foo[@@var2]
foo[$var1] = foo[$var2]
matrix[1, 2] = matrix[1, 3]
matrix[1, compute] = matrix[1, compute]
items[lookup] = items[lookup]

# Constant from different scope
Foo = ::Foo
