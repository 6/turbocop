def foo
  something
end

def bar
  class MyClass
    def inner_method
      work
    end
  end
end

def with_qualified_scope
  ::Class.new do
    def inner
      work
    end
  end
end

# Singleton method definitions (def obj.method) are allowed
def setup
  def @controller.new_method
    something
  end

  def self.class_method
    something
  end
end