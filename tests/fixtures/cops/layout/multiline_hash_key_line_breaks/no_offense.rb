x = {
  a: 1,
  b: 2,
  c: 3
}

y = { a: 1, b: 2, c: 3 }

z = {
  foo: :bar,
  baz: :qux
}

# All elements on same line in multiline hash (braces on different lines)
a = {
  type: 'number', required: true, example: 1
}

b = {
  one: 1, two: 2
}

# Multiline hash where each element gets its own line
c = {foo: 1,
  bar: {
    x: 1
  },
  baz: 2
}

# Element after multiline value on the closing line — no offense when the
# multiline element was itself already on a shared line (offending)
d = {:app => {},
  :settings => {:logger => ["/tmp/2.log"],
    :logger_level => 2},
  :defaults => {}
}
