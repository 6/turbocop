module Test
  extend self
  ^^^^^^^^^^^ Style/ModuleFunction: Use `module_function` instead of `extend self`.
  def test; end
end

module Foo
  extend self
  ^^^^^^^^^^^ Style/ModuleFunction: Use `module_function` instead of `extend self`.
  def bar; end
end

module Baz
  extend self
  ^^^^^^^^^^^ Style/ModuleFunction: Use `module_function` instead of `extend self`.
end
