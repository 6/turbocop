def foo(x, y = 1)
  return to_enum(__callee__, x)
         ^^^^^^^^^^^^^^^^^^^^^^ Lint/ToEnumArguments: Ensure you correctly provided all the arguments.
end

def bar(a, b, c)
  return to_enum(__method__, a)
         ^^^^^^^^^^^^^^^^^^^^^^ Lint/ToEnumArguments: Ensure you correctly provided all the arguments.
end

def baz(x, y)
  return enum_for(:baz, x)
         ^^^^^^^^^^^^^^^^^^ Lint/ToEnumArguments: Ensure you correctly provided all the arguments.
end

def select(*args, &block)
  return to_enum(:select) unless block_given?
         ^^^^^^^^^^^^^^^^^^^^ Lint/ToEnumArguments: Ensure you correctly provided all the arguments.
  args
end

# Argument mismatch: passes different variable than the parameter
def combination(n)
  num = n + 1
  return enum_for(:combination, num)
         ^^^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/ToEnumArguments: Ensure you correctly provided all the arguments.
end

# Swapped arguments
def process(x, y = 1)
  return to_enum(:process, y, x) unless block_given?
         ^^^^^^^^^^^^^^^^^^^^^^^^ Lint/ToEnumArguments: Ensure you correctly provided all the arguments.
end

# Missing optional arg
def recode(dt = nil, &block)
  return to_enum(:recode) unless block_given?
         ^^^^^^^^^^^^^^^^^^^^ Lint/ToEnumArguments: Ensure you correctly provided all the arguments.
end

# Missing keyword arg
def build(x, y = 1, *args, required:)
  return to_enum(:build, x, y, *args) unless block_given?
         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/ToEnumArguments: Ensure you correctly provided all the arguments.
end

# Wrong keyword value
def render(required:, optional: true)
  return to_enum(:render, required: something_else, optional: optional) unless block_given?
         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/ToEnumArguments: Ensure you correctly provided all the arguments.
end

# Missing splat keyword arg
def setup(x, y = 1, *args, required:, optional: true, **kwargs)
  return to_enum(:setup, x, y, *args, required: required, optional: optional) unless block_given?
         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/ToEnumArguments: Ensure you correctly provided all the arguments.
end

# self.to_enum with missing args
def transform(x)
  return self.to_enum(:transform) unless block_given?
         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/ToEnumArguments: Ensure you correctly provided all the arguments.
end

# Multiple params, only method name passed (no args at all)
def each_heredoc_node(node, parents, &block)
  enum_for(:each_heredoc_node, node)
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/ToEnumArguments: Ensure you correctly provided all the arguments.
end
