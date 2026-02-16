x ? x : y
^^ Style/RedundantCondition: Use double pipes `||` instead.

if a
^^ Style/RedundantCondition: Use double pipes `||` instead.
  a
else
  b
end

if foo
^^ Style/RedundantCondition: Use double pipes `||` instead.
  foo
else
  bar
end
