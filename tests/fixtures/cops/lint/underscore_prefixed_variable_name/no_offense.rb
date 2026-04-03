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

# Variable assigned in block but never read (no cross-block leaking)
describe 'records' do
  it 'does something' do
    _unused_record = create(:record)
    expect(1).to eq(1)
  end

  it 'does something else' do
    _unused_record = create(:record)
    expect(2).to eq(2)
  end
end

# Variable inside a module block, not read
module Config
  setup do
    _temp = 42
  end
end

# Underscore var assigned inside a lambda but not read
def setup_workspace
  handler = ->{ _temp = 42 }
  handler.call
end

# Rescue exception capture not read
def safe_operation
  begin
    risky
  rescue StandardError => _e
    puts "error"
  end
end

# Pattern match variable not read
def classify(value)
  case value
  in _x
    "matched"
  end
end
