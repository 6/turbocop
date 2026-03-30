if !x
^^^^^ Style/NegatedIfElseCondition: Invert the negated condition and swap the if-else branches.
  do_something
else
  do_something_else
end

if not y
^^^^^^^^ Style/NegatedIfElseCondition: Invert the negated condition and swap the if-else branches.
  a
else
  b
end

!z ? do_something : do_something_else
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/NegatedIfElseCondition: Invert the negated condition and swap the ternary branches.

# != operator is a negated condition
if x != y
^^^^^^^^^ Style/NegatedIfElseCondition: Invert the negated condition and swap the if-else branches.
  do_something
else
  do_something_else
end

# !~ operator is a negated condition
if x !~ y
^^^^^^^^^ Style/NegatedIfElseCondition: Invert the negated condition and swap the if-else branches.
  do_something
else
  do_something_else
end

# != in ternary
x != y ? do_something : do_something_else
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/NegatedIfElseCondition: Invert the negated condition and swap the ternary branches.

# Parenthesized negated condition
if (!x)
^^^^^^^ Style/NegatedIfElseCondition: Invert the negated condition and swap the if-else branches.
  do_something
else
  do_something_else
end

# Parenthesized != condition
if (x != y)
^^^^^^^^^^^ Style/NegatedIfElseCondition: Invert the negated condition and swap the if-else branches.
  do_something
else
  do_something_else
end

# begin-end wrapped negated condition
if begin
^^^^^^^^ Style/NegatedIfElseCondition: Invert the negated condition and swap the if-else branches.
  x != y
end
  do_something
else
  do_something_else
end

# Empty if-branch with negated condition
if !condition.nil?
^^^^^^^^^^^^^^^^^^ Style/NegatedIfElseCondition: Invert the negated condition and swap the if-else branches.
else
  foo = 42
end

unless !File.exists?(src)
^^^^^^^^^^^^^^^^^^^^^^^^^ Style/NegatedIfElseCondition: Invert the negated condition and swap the if-else branches.
  upload(src)
else
  missing(src)
end

unless !File.exists?(full_scriptname)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/NegatedIfElseCondition: Invert the negated condition and swap the if-else branches.
  transfer(full_scriptname)
else
  raise "missing"
end
