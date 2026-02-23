def m(&block)
  items.something(&block)
end

def n
  items.something { |i, j| yield j, i }
end

def o
  items.something { |i| do_something(i) }
end

def p
  yield
end
