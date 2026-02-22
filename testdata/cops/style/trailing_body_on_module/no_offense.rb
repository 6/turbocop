module Foo
  extend self
end

module Bar
  include Baz
end

module Qux
  def foo; end
end

module Empty; end

module ::TopLevel; def foo; '1'; end; end
module ::Other; def bar; '2'; end; end
