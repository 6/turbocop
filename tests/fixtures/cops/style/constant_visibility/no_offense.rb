class Foo
  BAR = 42
  private_constant :BAR
end

class Baz
  QUX = 42
  public_constant :QUX
end

TOPLEVEL = 1
x = 1

Foo::BAR = 1

module Proto
  Trace::CachePolicy = 1
  private_constant :CachePolicy
end
