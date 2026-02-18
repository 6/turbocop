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
# include used as RSpec matcher (not at class level)
expect(foo).to include(Bar, Baz)
include Foo, Bar
