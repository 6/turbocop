if x < low
^^^^^^^^^^ Style/ComparableClamp: Use `clamp` instead of `if/elsif/else`.
  low
elsif high < x
  high
else
  x
end

if low > x
^^^^^^^^^^ Style/ComparableClamp: Use `clamp` instead of `if/elsif/else`.
  low
elsif high < x
  high
else
  x
end

if x < low
^^^^^^^^^^ Style/ComparableClamp: Use `clamp` instead of `if/elsif/else`.
  low
elsif x > high
  high
else
  x
end
