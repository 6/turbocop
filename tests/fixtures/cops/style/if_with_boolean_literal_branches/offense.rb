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

# elsif with boolean literal branches
if foo
  true
elsif bar.empty?
^^^^^^ Style/IfWithBooleanLiteralBranches: Use `else` instead of redundant `elsif` with boolean literal branches.
  true
else
  false
end

# Complex || chain of predicate methods
(a.present? || b.present? || c.present?) ? true : false
                                         ^^^^^^^^^^^^^^^ Style/IfWithBooleanLiteralBranches: Remove redundant ternary operator with boolean literal branches.

# and keyword with boolean-returning right operand
(foo and bar == "baz") ? true : false
                       ^^^^^^^^^^^^^^^ Style/IfWithBooleanLiteralBranches: Remove redundant ternary operator with boolean literal branches.
