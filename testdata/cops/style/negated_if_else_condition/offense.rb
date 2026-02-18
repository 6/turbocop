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
