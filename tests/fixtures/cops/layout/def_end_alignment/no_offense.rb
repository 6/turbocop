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

# Modifier before def: end aligns with line start, not def keyword
private_class_method def self.helper(x)
  x + 1
end
