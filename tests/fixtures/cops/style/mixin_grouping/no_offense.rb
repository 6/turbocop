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
# Multi-arg include inside class << self should not be flagged
# (RuboCop does not define on_sclass, only on_class and on_module)
class Puppet::Provider
  class << self
    include Puppet::Util, Puppet::Util::Docs
    if n
    end
  end
end
class SomeType
  class << self
    include Enumerable, ClassGen
  end
end
