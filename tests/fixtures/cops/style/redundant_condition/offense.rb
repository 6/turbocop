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

x.nil? ? true : x
^^^^^^ Style/RedundantCondition: Use double pipes `||` instead.

if a.empty?
^^ Style/RedundantCondition: Use double pipes `||` instead.
  true
else
  a
end

# unless with condition == else_branch
unless b
^^^^^^ Style/RedundantCondition: Use double pipes `||` instead.
  y(x, z)
else
  b
end

# no-else pattern: if cond; cond; end → "This condition is not needed."
if do_something
^^ Style/RedundantCondition: This condition is not needed.
  do_something
end

# assignment branches: both branches assign to same variable
if foo
^^ Style/RedundantCondition: Use double pipes `||` instead.
  @value = foo
else
  @value = 'bar'
end

# local variable assignment branches
if foo
^^ Style/RedundantCondition: Use double pipes `||` instead.
  value = foo
else
  value = 'bar'
end

# method call branches with same receiver
if x
^^ Style/RedundantCondition: Use double pipes `||` instead.
  X.find(x)
else
  X.find(y)
end

# ternary with method call condition
b.x ? b.x : c
^^^^ Style/RedundantCondition: Use double pipes `||` instead.

# ternary with function call condition
a = b(x) ? b(x) : c
    ^^^^ Style/RedundantCondition: Use double pipes `||` instead.

# ternary predicate+true with number else
a.zero? ? true : 5
^^^^^^^ Style/RedundantCondition: Use double pipes `||` instead.
