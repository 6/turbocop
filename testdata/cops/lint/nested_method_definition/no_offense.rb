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