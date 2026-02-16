class Foo
end

module Bar
end

class Foo
  class Bar
  end
end

module FooModule
  module BarModule
  end
end

class FooClass
  class BarClass
  end
end

# Class inside class with inheritance (common pattern in policy objects)
class InboxPolicy < ApplicationPolicy
  class Scope
    def resolve
      super
    end
  end
end

# Module inside class (nested style is fine)
class MyService
  module Helpers
    def help; end
  end
end
