# Normal variable usage
def some_method
  foo = 1
  puts foo
end

# Underscore-prefixed variable that is only assigned (not read)
def another_method
  _unused = 1
  _unused = 2
end

# Normal parameter
def third_method(bar)
  puts bar
end

# Bare underscore is always OK
def fourth_method(_)
  puts _
end

# Variable captured and reassigned by block (not a reference)
_captured = 1
1.times do
  _captured = 2
end

# Unused underscore-prefixed method param
def unused_param(_data)
  42
end

# Forwarding with bare super
def forwarded(*_args)
  super
end

# Forwarding with binding
def bound(*_args)
  binding
end

# Block keyword arguments with AllowKeywordBlockArguments (default true)
items.each do |_name:, _value:|
  puts "processing"
end

# Multi-assignment where underscore vars are not read
def multi_unused
  _a, _b = 1, 2
end

# Block-pass parameter that is not read
def no_invoke(&_block)
  42
end

# Keyword rest parameter that is not read
def no_opts(**_opts)
  42
end

# For-loop variable not read in body
def skip_items(items)
  for _item in items
    process
  end
end

# Bare underscore not read
def ignore_arg(_)
  42
end

# Named capture not read
def match_only(str)
  /(?<_capture>\w+)/ =~ str
end
