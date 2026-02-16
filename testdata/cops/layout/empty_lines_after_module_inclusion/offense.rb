class Foo
  include Bar
  ^^^^^^^^^^^ Layout/EmptyLinesAfterModuleInclusion: Add an empty line after module inclusion.
  attr_reader :baz
end

class Qux
  extend ActiveSupport::Concern
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Layout/EmptyLinesAfterModuleInclusion: Add an empty line after module inclusion.
  def some_method
  end
end

class Abc
  prepend MyModule
  ^^^^^^^^^^^^^^^^ Layout/EmptyLinesAfterModuleInclusion: Add an empty line after module inclusion.
  def another_method
  end
end
