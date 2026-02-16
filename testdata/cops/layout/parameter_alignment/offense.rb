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
