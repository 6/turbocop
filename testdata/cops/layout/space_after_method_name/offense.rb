def func (x)
        ^ Layout/SpaceAfterMethodName: Do not put a space between a method name and the opening parenthesis.
  x
end

def method= (y)
           ^ Layout/SpaceAfterMethodName: Do not put a space between a method name and the opening parenthesis.
  @y = y
end

def bar (a, b)
       ^ Layout/SpaceAfterMethodName: Do not put a space between a method name and the opening parenthesis.
  a + b
end
