if x
^^ Style/IfUnlessModifier: Favor modifier `if` usage when having a single-line body.
  do_something
end

unless x
^^^^^^ Style/IfUnlessModifier: Favor modifier `unless` usage when having a single-line body.
  do_something
end

if condition
^^ Style/IfUnlessModifier: Favor modifier `if` usage when having a single-line body.
  foo
end

unless finished?
^^^^^^ Style/IfUnlessModifier: Favor modifier `unless` usage when having a single-line body.
  retry
end
