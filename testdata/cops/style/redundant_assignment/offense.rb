def func
  some_preceding_statements
  x = something
  ^^^^^^^^^^^^^ Style/RedundantAssignment: Redundant assignment before returning detected.
  x
end

def bar
  y = compute
  ^^^^^^^^^^^ Style/RedundantAssignment: Redundant assignment before returning detected.
  y
end

def baz
  result = a + b
  ^^^^^^^^^^^^^^ Style/RedundantAssignment: Redundant assignment before returning detected.
  result
end
