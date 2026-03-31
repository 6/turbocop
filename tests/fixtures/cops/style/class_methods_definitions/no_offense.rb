class A
  def self.three
  end
end

class B
  class << self
    attr_reader :two
  end
end

class C
  def self.foo
    42
  end
end

# All private methods inside class << self (standalone private)
class D
  class << self
    private

    def helper
      42
    end
  end
end

# Inline private def
class E
  class << self
    private def secret
      42
    end
  end
end

# All protected methods
class F
  class << self
    protected

    def internal
      42
    end
  end
end

# Mixed private and protected, no public
class G
  class << self
    private

    def helper_one
      1
    end

    protected

    def helper_two
      2
    end
  end
end

# Inline protected def
class H
  class << self
    protected def guarded
      42
    end
  end
end

# Inline public def only — no direct child plain def nodes
class T
  class << self
    public def visible
      42
    end
  end
end

# Mixed public and protected — not ALL methods are public
class I
  class << self
    def address
      "ak_123"
    end

    protected

    def rand_strings
      "abc"
    end
  end
end

# Mixed public and private — not ALL methods are public
class J
  class << self
    def visible
      42
    end

    private

    def helper
      1
    end
  end
end

# private then public restores visibility but private def remains
class K
  class << self
    private

    def helper
    end

    public

    def visible
    end
  end
end

# private :method_name after def marks method as non-public
class L
  class << self
    def my_class_method
      :original_return_value
    end
    private :my_class_method
  end
end

# protected :method_name after def marks method as non-public
class M
  class << self
    def my_class_method
      :value
    end
    protected :my_class_method
  end
end

# Multiple methods all made private via symbol args
class N
  class << self
    def foo
      1
    end

    def bar
      2
    end

    private :foo, :bar
  end
end

# Mix of public def and private :name — not all public
class O
  class << self
    def visible
      42
    end

    def hidden
      1
    end
    private :hidden
  end
end

# Single-line class << self; def ...; end; end — RuboCop does not flag
class P; class << self; def mug; end; end; end
Class.new { class << self; def meth; 1; end; end }.new
@class = Class.new { class << self; def meth; 1; end; end }

# Only def self.x inside class << self — no plain def
class Q
  class << self
    def self.x
    end
  end
end

# class << not_self — not a self receiver
class R
  class << new.bar
    def f; end
  end
end

# alias + def with explicit receiver (not a plain def)
class S
  class << self
    alias os_trap trap
    def Signal.trap(sig, &block)
    end
  end
end
