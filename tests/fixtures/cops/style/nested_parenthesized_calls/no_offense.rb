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

# Operator methods inside parenthesized calls are not nested method calls
assert(cdir1 != cdir3)
assert(a == b)
expect(x >= y)
method1(a + b)
method1(a <=> b)
method1(a =~ b)
method1(a !~ b)
method1(a << b)
method1(a ** b)

# Nested calls with real blocks are excluded
write(render_view_component(Primer::OpenProject::CollapsibleSection.new(id: 1)) { |section| section.with_title { :ok } })
