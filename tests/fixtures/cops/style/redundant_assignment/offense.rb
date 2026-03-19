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

# Inside if/elsif/else branches
def with_if_branches
  some_preceding_statements
  if x
    z = 1
    ^^^^^ Style/RedundantAssignment: Redundant assignment before returning detected.
    z
  elsif y
    2
  else
    z = 3
    ^^^^^ Style/RedundantAssignment: Redundant assignment before returning detected.
    z
  end
end

# Inside case/when branches
def with_case_when
  some_preceding_statements
  case x
  when y
    res = 1
    ^^^^^^^ Style/RedundantAssignment: Redundant assignment before returning detected.
    res
  when z
    2
  else
    res = 3
    ^^^^^^^ Style/RedundantAssignment: Redundant assignment before returning detected.
    res
  end
end

# Inside case/in (pattern matching) branches
def with_case_in
  case x
  in y
    res = 1
    ^^^^^^^ Style/RedundantAssignment: Redundant assignment before returning detected.
    res
  in z
    2
  else
    res = 3
    ^^^^^^^ Style/RedundantAssignment: Redundant assignment before returning detected.
    res
  end
end

# Inside explicit begin..end block
def with_begin
  some_preceding_statements
  begin
    x = something
    ^^^^^^^^^^^^^ Style/RedundantAssignment: Redundant assignment before returning detected.
    x
  end
end

# Inside rescue blocks
def with_rescue
  1
  x = 2
  ^^^^^ Style/RedundantAssignment: Redundant assignment before returning detected.
  x
rescue SomeException
  3
  x = 4
  ^^^^^ Style/RedundantAssignment: Redundant assignment before returning detected.
  x
rescue AnotherException
  5
end

# Class method (defs node)
def self.class_method
  x = something
  ^^^^^^^^^^^^^ Style/RedundantAssignment: Redundant assignment before returning detected.
  x
end
