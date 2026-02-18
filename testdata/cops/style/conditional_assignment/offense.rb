if condition
^^^^^^^^^^^^ Style/ConditionalAssignment: Use the return value of `if` expression for variable assignment and comparison.
  x = 1
else
  x = 2
end

if foo
^^^^^^ Style/ConditionalAssignment: Use the return value of `if` expression for variable assignment and comparison.
  bar = something
else
  bar = other_thing
end

if test
^^^^^^^ Style/ConditionalAssignment: Use the return value of `if` expression for variable assignment and comparison.
  result = :yes
else
  result = :no
end
