obj.hash
"string".hash
x.hash
42.hash
{}.hash

# Multi-element array .hash is the recommended pattern
[a, b].hash
[x, y, z].hash
[1, 2].hash
[@foo, @bar].hash

# Inside a def hash method — this is the correct implementation pattern
def hash
  [control_node.object_id, control_node.object_id].hash
end

# XOR outside def hash method — not an offense
def something
  a ^ b
end

# Normal bitwise ops outside hash method
flags = x ^ y
result = a + b
mask = x | y

# Delegating to a single object inside def hash
def hash
  1.hash
end

# Multi-element array without .hash on elements inside def hash
def hash
  [1, 2, 3].hash
end

# def hash with parameters — this is a custom method, not Object#hash
def self.hash(uin, ptwebqq)
  n = Array.new(4, 0)
  v = uin >> 24 & 255 ^ uin
  v + 1
end

# Another method named hash with args — not Object#hash
def hash(data)
  buffer = prefix
  buffer += data
  buffer
end
