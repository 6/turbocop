class Foo
  include Bar
  include Qux
end
class Baz
  extend A
  extend B
end
class Quux
  prepend X
end
expect(foo).to include(bar: baz)
