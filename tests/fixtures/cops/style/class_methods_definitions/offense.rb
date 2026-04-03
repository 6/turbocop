class A
  class << self
  ^^^^^^^^^^^^^ Style/ClassMethodsDefinitions: Do not define public methods within class << self.
    def three
    end
  end
end

class B
  class << self
  ^^^^^^^^^^^^^ Style/ClassMethodsDefinitions: Do not define public methods within class << self.
    def foo
    end

    def bar
    end
  end
end

class C
  class << self
  ^^^^^^^^^^^^^ Style/ClassMethodsDefinitions: Do not define public methods within class << self.
    attr_reader :two

    def three
    end
  end
end

# private :new + def other — other is still public
class D
  class << self
  ^^^^^^^^^^^^^ Style/ClassMethodsDefinitions: Do not define public methods within class << self.
    private :new

    def of_raw_data(site)
      42
    end
  end
end

# protected :new + def wrap — wrap is still public
class E
  class << self
  ^^^^^^^^^^^^^ Style/ClassMethodsDefinitions: Do not define public methods within class << self.
    protected :new

    def wrap(o, c)
      42
    end
  end
end

# include + def — include doesn't affect visibility
class F
  class << self
  ^^^^^^^^^^^^^ Style/ClassMethodsDefinitions: Do not define public methods within class << self.
    include Foo

    def bar
      42
    end
  end
end

# attr_reader + private :new + def — def is still public
class G
  class << self
  ^^^^^^^^^^^^^ Style/ClassMethodsDefinitions: Do not define public methods within class << self.
    attr_reader :registered_plugins
    private :new

    def def_field(*names)
      42
    end
  end
end

# private :name before def name — def name redefines as public
class H
  class << self
  ^^^^^^^^^^^^^ Style/ClassMethodsDefinitions: Do not define public methods within class << self.
    private :next_migration_number

    def next_migration_number(dir)
      42
    end
  end
end

# inline private def does not count as a plain def child
class I
  class << self
  ^^^^^^^^^^^^^ Style/ClassMethodsDefinitions: Do not define public methods within class << self.
    private def helper
      1
    end

    def visible
      2
    end
  end
end

# inline protected def does not count as a plain def child
class J
  class << self
  ^^^^^^^^^^^^^ Style/ClassMethodsDefinitions: Do not define public methods within class << self.
    protected def guarded
      1
    end

    def visible
      2
    end
  end
end

# later inline private def does not make earlier direct defs non-public
class K
  class << self
  ^^^^^^^^^^^^^ Style/ClassMethodsDefinitions: Do not define public methods within class << self.
    def visible
      42
    end

    private def helper
      1
    end
  end
end

# direct defs stay countable even with accessor calls and inline private defs
class L
  class << self
  ^^^^^^^^^^^^^ Style/ClassMethodsDefinitions: Do not define public methods within class << self.
    attr_writer :console

    private def console?
      @console ||= false
    end

    def debug(message)
      message
    end

    def info(message)
      message
    end
  end
end

# constants + defs inside class << self — defs are still public
module Outer
  module Inner
    class << self
    ^^^^^^^^^^^^^ Style/ClassMethodsDefinitions: Do not define public methods within class << self.
      SIDE = 0.1
      LIMIT = 5

      def compute(x, y)
        42
      end
    end
  end
end

# multi-arg private :foo, :bar — RuboCop does not recognize multi-arg form
class MultiPrivate
  class << self
  ^^^^^^^^^^^^^ Style/ClassMethodsDefinitions: Do not define public methods within class << self.
    def foo
      1
    end

    def bar
      2
    end

    private :foo, :bar
  end
end

# multi-arg protected with remaining public def
class MultiProtected
  class << self
  ^^^^^^^^^^^^^ Style/ClassMethodsDefinitions: Do not define public methods within class << self.
    def format(table)
      42
    end

    def helper(x)
      43
    end

    protected :helper, :format
  end
end
