class Foo
  def bar
    42
  end
end

class Baz < Base
  include Something
end

class Qux
  attr_reader :name
end
