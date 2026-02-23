def foo
  return 42
  ^^^^^^^^^ Style/RedundantReturn: Redundant `return` detected.
end

def bar
  x = 1
  return x
  ^^^^^^^^ Style/RedundantReturn: Redundant `return` detected.
end

def baz(x)
  return x + 1
  ^^^^^^^^^^^^ Style/RedundantReturn: Redundant `return` detected.
end
