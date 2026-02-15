x = 1
y = 2
z = "has;semicolon"
w = 'also;has;one'
a = "multi #{x}; value"
# comment; not code

# Single-line bodies (handled by other cops, not Style/Semicolon)
def show; end
def foo; bar; end
class EmptyError < StandardError; end
module Mixin; end
