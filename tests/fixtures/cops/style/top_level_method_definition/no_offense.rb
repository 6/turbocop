class Foo
  def bar
    'baz'
  end

  define_method(:baz) { 1 }
  XDR::Union.define_method(:qux, instance_method(:bar))
end

module Helper
  def help
    true
  end
end

x = 1
