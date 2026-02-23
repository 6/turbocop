# Keyword boolean argument is fine
def some_method(bar: false)
  puts bar
end

# No arguments
def some_method
  puts "hello"
end

# Non-boolean optional argument
def some_method(bar = 'foo')
  puts bar
end

# Allowed method: respond_to_missing?
def respond_to_missing?(method, include_all = false)
  super
end

# Integer default
def some_method(bar = 42)
  puts bar
end
