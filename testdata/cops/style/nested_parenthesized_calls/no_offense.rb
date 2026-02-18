puts(compute(something))
puts 1, 2
puts
method(obj[1])
foo(bar(baz))
expect(foo).to be(bar)

# Setter methods are excluded
method(obj.attr = value)
