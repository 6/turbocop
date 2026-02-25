def foo(name:, age: nil)
end
def bar(**options)
end
def baz(config = nil)
end
def qux(data = [])
end
def quux(options = "default")
end
# Method with super call should be skipped
def update(options = {})
  super
end
def process(opts = {})
  super(opts)
end
# optarg not last param: followed by block
def build(options = {}, &block)
  block.call(options)
end
# optarg not last param: followed by keyword args
def create(options = {}, name:)
  name
end
# optarg not last param: followed by rest
def invoke(options = {}, *rest)
  rest
end
