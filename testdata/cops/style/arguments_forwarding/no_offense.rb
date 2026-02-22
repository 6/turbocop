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

# Block arg name 'task' is not in RedundantBlockArgumentNames — meaningful name
# AllowOnlyRestArgument prevents suggesting ... forwarding
def post(*args, &task)
  @executor&.post(*args, &task)
end

# args referenced directly (not just in *args splat) — cannot use ...
def capture(*args, &block)
  buf.capture(*args, &block)
  args.first
end

# block referenced directly (block.call) — cannot use ...
def wrap(*args, &block)
  run(*args, &block)
  block.call
end
