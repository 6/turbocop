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
