def foo(x, y = 1)
  return to_enum(__callee__, x, y)
end

def bar(a, b)
  return to_enum(__method__, a, b)
end

def baz
  return to_enum(:baz)
end
