x = if condition
^^^^^^^^^^^^^^^^^^^^ Style/RedundantSelfAssignmentBranch: Redundant self-assignment branch. The variable `x` is assigned to itself in one of the branches.
  do_something
else
  x
end

x = condition ? do_something : x
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/RedundantSelfAssignmentBranch: Redundant self-assignment branch. The variable `x` is assigned to itself in one of the branches.

x = condition ? x : do_something
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/RedundantSelfAssignmentBranch: Redundant self-assignment branch. The variable `x` is assigned to itself in one of the branches.
