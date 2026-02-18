a = "test"
a == "x" || a == "y"
^^^^^^^^^^^^^^^^^^^^ Style/MultipleComparison: Avoid comparing a variable with multiple items in a conditional, use `Array#include?` instead.

a == "x" || a == "y" || a == "z"
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/MultipleComparison: Avoid comparing a variable with multiple items in a conditional, use `Array#include?` instead.

"x" == a || "y" == a
^^^^^^^^^^^^^^^^^^^^ Style/MultipleComparison: Avoid comparing a variable with multiple items in a conditional, use `Array#include?` instead.

# With method call comparisons mixed in (method calls are skipped but non-method comparisons count)
a == foo.bar || a == 'x' || a == 'y'
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/MultipleComparison: Avoid comparing a variable with multiple items in a conditional, use `Array#include?` instead.
