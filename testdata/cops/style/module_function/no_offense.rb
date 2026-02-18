module Test
  module_function
  def test; end
end
class Foo
  extend self
end
module Bar
  extend SomeModule
end
module Baz
  module_function :test
end
