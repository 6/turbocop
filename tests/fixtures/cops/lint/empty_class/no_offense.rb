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

# Classes with a superclass are allowed even when empty
class Error < StandardError
end

class NotFound < HttpError
end

class MyError < RuntimeError; end
