foo.size
bar.count
qux.length
CONST = 'a'.size
CONST = [1, 2, 3].length
CONST = {a: 1, b: 2}.size
[1, 2, *foo].count
{a: 1, **foo}.length
:"#{fred}".size
foo = "abc"
foo.size
foo = [1, 2, 3]
foo.count
"foo".count(bar)
"foo".count(@bar)
[1, 2, 3].count { |v| v == 'a' }
[1, 2, 3].count(&:any?)
items.inject('foo'.length) { |acc, x| acc + x }
