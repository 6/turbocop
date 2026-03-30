def foo(
  bar,
  baz
)
end

def something(first, second)
end

def method_a(
  first,
  second
)
end

# Keyword params correctly indented
def method_b(
  foo: 1,
  bar: 2
)
  foo
end

# Block param correctly indented
def method_c(
  &block
)
  block
end

# Keyword rest correctly indented
def method_d(
  **kwargs
)
  kwargs
end

# Rest param correctly indented
def method_e(
  *args
)
  args
end

# No paren method def (should be ignored)
def no_parens foo, bar
  foo
end

# Single line (should be ignored)
def single_line(foo, bar)
  foo
end

# def self.method correctly indented
def self.class_method(
  first,
  second
)
  first
end

# Keyword-only params correctly indented
def keyword_only(
  name:,
  value:
)
  name
end

# First param on same line as paren (not checked by this cop)
def same_line(first,
              second)
  first
end

# Tab-indented modifier def correctly indented
	register_element def animate_transform(
	  **attributes,
		&content
	) = nil
