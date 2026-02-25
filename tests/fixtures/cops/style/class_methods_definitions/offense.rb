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

# private then public restores public visibility
class D
  class << self
  ^^^^^^^^^^^^^ Style/ClassMethodsDefinitions: Do not define public methods within class << self.
    private

    def helper
    end

    public

    def visible
    end
  end
end
