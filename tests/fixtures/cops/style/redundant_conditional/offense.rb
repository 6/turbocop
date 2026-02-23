x == y ? true : false
^^^^^^^^^^^^^^^^^^^^^ Style/RedundantConditional: This conditional expression can just be replaced by `x == y`.

if x == y
^^^^^^^^^ Style/RedundantConditional: This conditional expression can just be replaced by `x == y`.
  true
else
  false
end

x == y ? false : true
^^^^^^^^^^^^^^^^^^^^^ Style/RedundantConditional: This conditional expression can just be replaced by `!(x == y)`.
