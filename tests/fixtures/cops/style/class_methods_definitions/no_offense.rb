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
