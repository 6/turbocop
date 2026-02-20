puts(compute(something))
puts 1, 2
puts
method(obj[1])
foo(bar(baz))
expect(foo).to be(bar)

# Setter methods are excluded
method(obj.attr = value)

# Bracket indexer calls are not parenthesized calls
json[:key] = Routes.url_for self
hash[:a] = some_method arg1, arg2
