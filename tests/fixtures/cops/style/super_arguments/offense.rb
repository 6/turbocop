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

def with_rest(*args, **kwargs)
  super(*args, **kwargs)
  ^^^^^^^^^^^^^^^^^^^^^^ Style/SuperArguments: Call `super` without arguments and parentheses when the signature is identical.
end

def with_keyword(name:, age:)
  super(name: name, age: age)
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/SuperArguments: Call `super` without arguments and parentheses when the signature is identical.
end

def with_block(name, &block)
  super(name, &block)
  ^^^^^^^^^^^^^^^^^^^^ Style/SuperArguments: Call `super` without arguments and parentheses when the signature is identical.
end

def with_mixed(a, *args, b:)
  super(a, *args, b: b)
  ^^^^^^^^^^^^^^^^^^^^^^ Style/SuperArguments: Call `super` without arguments and parentheses when the signature is identical.
end
