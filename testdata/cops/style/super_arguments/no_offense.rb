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

# Forwarding only a subset of args
def with_kwargs(a, b:)
  super(a)
end

# Different rest args
def with_rest(a, *args)
  super(a)
end

# Different block
def with_block(a, &blk)
  super(a)
end

# Anonymous keyword rest — super doesn't forward keyword args
def initialize(app, **)
  super app
end
