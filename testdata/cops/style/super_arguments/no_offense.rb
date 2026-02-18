def foo(a, b)
  super
end

def bar(x, y)
  super(x)
end

def baz(a, b)
  super(b, a)
end
x = 1
y = 2

# With keyword arguments - not flagged
def with_kwargs(a, b:)
  super(a, b: b)
end

# With rest args - not flagged
def with_rest(a, *args)
  super(a, *args)
end

# With block - not flagged
def with_block(a, &blk)
  super(a, &blk)
end
