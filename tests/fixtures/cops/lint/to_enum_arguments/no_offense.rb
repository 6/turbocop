def foo(x, y = 1)
  return to_enum(__callee__, x, y)
end

def bar(a, b)
  return to_enum(__method__, a, b)
end

def baz
  return to_enum(:baz)
end

# All args match including keyword args
def process(x, y = 1, *args, required:, optional: true, **kwargs, &block)
  return to_enum(:process, x, y, *args, required: required, optional: optional, **kwargs)
end

# Enum for a different method (should not check args)
def compute(x)
  return to_enum(:other_method) unless block_given?
end

# No args method with no args call
def iterate
  return to_enum(:iterate) unless block_given?
end

# Method call has receiver other than self
def transform(x)
  return other.to_enum(:transform) unless block_given?
end

# Block arg is excluded from matching
def iterate(x, &block)
  return to_enum(:iterate, x) unless block_given?
end

# self.enum_for with correct args
def each(x)
  return self.enum_for(:each, x) unless block_given?
end

# Method with only keyword args, all matching
def query(name:, limit: 10)
  return to_enum(:query, name: name, limit: limit)
end

# Not calling current method — don't check args
def convert(x)
  return to_enum(:other) unless block_given?
end
