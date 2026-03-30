def foo(
      bar,
      ^^^ Layout/FirstParameterIndentation: Use 2 (not 6) spaces for indentation.
  baz
)
end

def method_a(
        first,
        ^^^^^ Layout/FirstParameterIndentation: Use 2 (not 8) spaces for indentation.
  second
)
end

def method_b(
    first,
    ^^^^^ Layout/FirstParameterIndentation: Use 2 (not 4) spaces for indentation.
  second
)
end

# Keyword parameters should also be checked
def method_c(
      foo: 1,
      ^^^^^^ Layout/FirstParameterIndentation: Use 2 (not 6) spaces for indentation.
  bar: 2
)
  foo
end

# Keyword-only params (no required params before them)
def method_d(
        name:,
        ^^^^^ Layout/FirstParameterIndentation: Use 2 (not 8) spaces for indentation.
  value:
)
  name
end

# Block parameter as first param
def method_e(
      &block
      ^^^^^^ Layout/FirstParameterIndentation: Use 2 (not 6) spaces for indentation.
)
  block
end

# Keyword rest as first param
def method_f(
      **kwargs
      ^^^^^^^^ Layout/FirstParameterIndentation: Use 2 (not 6) spaces for indentation.
)
  kwargs
end

# Post params (after rest) - rest is first param
def method_g(
      *args,
      ^^^^^ Layout/FirstParameterIndentation: Use 2 (not 6) spaces for indentation.
  post
)
  args
end

# def self.method (class method)
def self.method_h(
      first,
      ^^^^^ Layout/FirstParameterIndentation: Use 2 (not 6) spaces for indentation.
  second
)
  first
end

# Tab-indented modifier def still uses the def line as the base indentation
	register_element def animate_transform(
		**attributes,
		^^^^^^^^^^^^ Layout/FirstParameterIndentation: Use 3 (not 2) spaces for indentation.
		&content
	) = nil
