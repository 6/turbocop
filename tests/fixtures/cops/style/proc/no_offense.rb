f = proc { |x| x + 1 }

g = Object.new

h = proc { puts "hello" }

i = String.new

j = proc do |x|
  x * 2
end

k = ::Object.new

# Proc.new without a block (e.g., as default parameter) is fine
def define_action(name, handler = Proc.new)
  @actions[name] = handler
end

# Proc.new with block argument forwarding — not the same as a literal block
p = Proc.new(&block)
p2 = Proc.new(&:sym)
p3 = Proc.new(&method(:foo))
