class Foo
  attr_accessor :foo
  ^^^^^^^^^^^^^^^^^ Layout/EmptyLinesAroundAttributeAccessor: Add an empty line after attribute accessor.
  def do_something
  end
end

class Bar
  attr_reader :bar
  ^^^^^^^^^^^^^^^^ Layout/EmptyLinesAroundAttributeAccessor: Add an empty line after attribute accessor.
  def another_method
  end
end

class Baz
  attr_writer :baz
  ^^^^^^^^^^^^^^^^ Layout/EmptyLinesAroundAttributeAccessor: Add an empty line after attribute accessor.
  def yet_another
  end
end
