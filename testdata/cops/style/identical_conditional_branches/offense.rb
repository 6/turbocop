if condition
  do_x
  do_z
  ^^^^ Style/IdenticalConditionalBranches: Move `do_z` out of the conditional.
else
  do_y
  do_z
end
if foo
  bar
  result
  ^^^^^^ Style/IdenticalConditionalBranches: Move `result` out of the conditional.
else
  baz
  result
end
if x
  a = 1
  b
  ^ Style/IdenticalConditionalBranches: Move `b` out of the conditional.
else
  a = 2
  b
end
