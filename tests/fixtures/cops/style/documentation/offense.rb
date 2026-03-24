class Foo
^^^^^ Style/Documentation: Missing top-level documentation comment for `class`.
  def method
  end
end

module Bar
^^^^^^ Style/Documentation: Missing top-level documentation comment for `module`.
  def method
  end
end

class MyClass
^^^^^ Style/Documentation: Missing top-level documentation comment for `class`.
  def method
  end
end

module MyModule
^^^^^^ Style/Documentation: Missing top-level documentation comment for `module`.
  def method
  end
end

module Test
^^^^^^ Style/Documentation: Missing top-level documentation comment for `module`.
end

module MixedConcern
^^^^^^ Style/Documentation: Missing top-level documentation comment for `module`.
  extend ActiveSupport::Concern

  module ClassMethods
  ^^^^^^ Style/Documentation: Missing top-level documentation comment for `module`.
    def some_method
    end
  end
end

# FN fix: include with method call argument (not const) should still need docs
module Types
^^^^^^ Style/Documentation: Missing top-level documentation comment for `module`.
  include Dry::Types()
end

# FN fix: include with method chain should still need docs
class Base
^^^^^ Style/Documentation: Missing top-level documentation comment for `class`.
  include ActionDispatch::Routing::RouteSet.new.url_helpers
end
