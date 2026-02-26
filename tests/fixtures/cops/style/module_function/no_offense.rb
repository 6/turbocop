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
# extend self with private directive should not be flagged
module WithPrivate
  extend self
  def greet; end
  private
  def helper; end
end
# extend self with private :method_name should not be flagged
module WithPrivateMethod
  extend self
  def greet; end
  private :helper
  def helper; end
end
# extend self with private def should not be flagged
module WithPrivateDef
  extend self
  def greet; end
  private def helper; end
end
