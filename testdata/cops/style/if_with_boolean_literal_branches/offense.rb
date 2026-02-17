if foo == bar
^^ Style/IfWithBooleanLiteralBranches: Remove redundant `if` with boolean literal branches.
  true
else
  false
end

if foo.do_something?
^^ Style/IfWithBooleanLiteralBranches: Remove redundant `if` with boolean literal branches.
  true
else
  false
end

if foo.do_something?
^^ Style/IfWithBooleanLiteralBranches: Remove redundant `if` with boolean literal branches.
  false
else
  true
end

unless foo.do_something?
^^^^^^ Style/IfWithBooleanLiteralBranches: Remove redundant `unless` with boolean literal branches.
  false
else
  true
end

foo == bar ? true : false
           ^^^^^^^^^^^^^^^ Style/IfWithBooleanLiteralBranches: Remove redundant ternary operator with boolean literal branches.
