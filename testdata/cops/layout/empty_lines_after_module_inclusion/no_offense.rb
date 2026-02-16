class Foo
  include Bar

  attr_reader :baz
end

class Baz
  extend ActiveSupport::Concern
  include Enumerable
  prepend MyModule

  def some_method
  end
end

class Simple
  include Comparable
end
