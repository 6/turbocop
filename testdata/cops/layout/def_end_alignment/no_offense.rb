def foo
  42
end

def bar = 42

def baz(x)
  x + 1
end

# Single-line defs should not be flagged
def qux; 42; end
def quux(x); x + 1; end
