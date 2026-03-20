MAX_SIZE = 100
VERSION = "1.0"
MyClass = Class.new
Foo = Struct.new(:bar)
TIMEOUT_IN_SECONDS = 30

# Constant-to-constant assignment (aliasing)
Server = BaseServer
Stream = Sinatra::Helpers::Stream

# Method call with non-literal receiver
Uchar1max = (1 << 7) - 1

# Receiverless method call
Config = setup_config

# Lambda-style method call (proc wraps in block node — allowed by RuboCop)
MyProc = proc { do_something }

# Compound assignment with SCREAMING_SNAKE_CASE (allowed)
COUNTER &&= 1
TOTAL += 10
Mod::LIMIT &&= 5
Mod::OFFSET += 1

# Rescue with SCREAMING_SNAKE_CASE constant target
begin
  something
rescue => LAST_ERROR
end

# CallNode with block — equivalent to Parser's :block type (always allowed)
Icons = { note: "info" }.transform_values { |v| v.upcase }
Items = [1, 2, 3].map { |x| x * 2 }
Config = %w[a b c].each_with_object({}) do |item, hash|
  hash[item] = true
end
