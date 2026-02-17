# Single top-level class with many nested/module-scoped classes is fine
class Foo
  def do_something
    puts "hello"
  end

  class Nested
    def bar
      "inner"
    end
  end
end

# Multiple classes inside a module are fine (not top-level)
module Mastodon
  class Error < StandardError; end
  class NotPermittedError < Error; end
  class ValidationError < Error; end
  class HostValidationError < ValidationError; end
end

# Class inside a nested module
module Outer
  module Inner
    class Widget
      def call
        true
      end
    end
  end
end

# Class inside a block (e.g. RSpec describe)
1.times do
  class TestHelper
    def help; end
  end
end

# Class inside a def
def make_class
  Class.new do
    def call; end
  end
end

# Class inside singleton class
class << self
  def factory
    "factory"
  end
end
