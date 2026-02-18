def foo(...)
  bar(...)
end

def bar(x, *args, &block)
  baz(*args, &block)
end

def baz(x, y)
  qux(x, y)
end

def test
  42
end

# *args and &block used in different calls — cannot use ...
def self.with(*args, &block)
  new(*args).tap(&block).to_element
end
