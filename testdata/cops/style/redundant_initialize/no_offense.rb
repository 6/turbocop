def initialize
  # initializer comment
end

def initialize(a, b)
  do_something
end

def initialize(a, b)
  super
  do_something
end

def do_something
end

def initialize(a, b)
  super()
end

def initialize(a, b = 5)
  super
end

def initialize(*args)
  super
end

def initialize(**kwargs)
  super
end

# Empty initialize with parameter — not redundant (overrides parent)
def initialize(_assistant); end
def initialize(arg)
end
