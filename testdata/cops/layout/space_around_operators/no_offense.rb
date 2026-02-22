x = 1
x == ""
x != y
a => "hello"
{a: 1, b: 2}
x += 1
"hello=world"
# x=1 inside comment
x = "a==b"

# Default parameters (handled by SpaceAroundEqualsInParameterDefault)
def foo(bar=1)
end
def baz(x=1, y=2)
end

# Spaceship operator (<=>) should not trigger => check
x <=> y
[1, 2, 3].sort { |a, b| a <=> b }

# Operator method definitions should not be flagged
def ==(other)
  id == other.id
end

def !=(other)
  !(self == other)
end

def []=(key, value)
  @data[key] = value
end

def <=>(other)
  name <=> other.name
end

def self.===(other)
  other.is_a?(self)
end

def >=(other)
  value >= other.value
end

# Safe navigation with operator method: &.!=
table_name&.!= node.left.relation.name

# Method call with dot before operator
x.== y

# Binary operators with proper spacing
x + y
x - y
x * y
x / y
x % y
x & y
x | y
x ^ y
x << y
x >> 1
x && y
x || y
x < y
x > y
x <= y
x >= y
x <=> y

# Unary operators (not binary — should not be flagged)
z = -x
z = +x

# Exponent operator with no_space style (default) should not be flagged
x = 2**10
y = n**(k - 1)

# AllowForAlignment: operators aligned across adjacent lines
title  = data[:title]  || ''
url    = data[:url]    || ''
width  = data[:width]  || 0
height = data[:height] || 0

# Trailing spaces before comment after operator — not flagged
x ||  # fallback
  y
a &&  # condition check
  b

# Operator at start of line (continuation) — indentation, not extra spacing
result = foo \
  + bar
x = a \
    || b
