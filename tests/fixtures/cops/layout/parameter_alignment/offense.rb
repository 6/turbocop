def foo(bar,
     baz)
     ^^^ Layout/ParameterAlignment: Align the parameters of a method definition if they span more than one line.
  123
end

def method_a(x,
      y)
      ^ Layout/ParameterAlignment: Align the parameters of a method definition if they span more than one line.
  x + y
end

def method_b(a,
        b)
        ^ Layout/ParameterAlignment: Align the parameters of a method definition if they span more than one line.
  a + b
end

# Misaligned block parameter
def bidi_streamer(method, requests, marshal, unmarshal,
                  deadline: nil,
                  return_op: false,
                  parent: nil,
                  credentials: nil,
                  metadata: {},
  &blk)
  ^^^^ Layout/ParameterAlignment: Align the parameters of a method definition if they span more than one line.
  blk.call
end

# Misaligned block parameter - simple case
def process(x,
            y,
  &block)
  ^^^^^^ Layout/ParameterAlignment: Align the parameters of a method definition if they span more than one line.
  block.call(x, y)
end

# Misaligned block parameter - another case
def handle(a,
           b,
       &blk)
       ^^^^ Layout/ParameterAlignment: Align the parameters of a method definition if they span more than one line.
  blk.call
end
