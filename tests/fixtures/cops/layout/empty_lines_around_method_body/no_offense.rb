def foo
  bar
end

def baz = 42

def qux(x)
  x + 1
end

# Multiline endless methods without blank lines
def greet(name,
  greeting) = "#{greeting}, #{name}"

def compute(value,
  factor) =
  value * factor

def self.get_single_choice(message, caption, choices, parent = nil,
                           initial_selection: 0,
                           pos: Wx::DEFAULT_POSITION) end
# Get the user selection as an index.
