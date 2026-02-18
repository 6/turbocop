def foo(a, b)
  super(a, b)
  ^^^^^^^^^^^ Style/SuperArguments: Call `super` without arguments and parentheses when the signature is identical.
end

def bar(x, y)
  super(x, y)
  ^^^^^^^^^^^ Style/SuperArguments: Call `super` without arguments and parentheses when the signature is identical.
end

def baz(name)
  super(name)
  ^^^^^^^^^^^ Style/SuperArguments: Call `super` without arguments and parentheses when the signature is identical.
end
