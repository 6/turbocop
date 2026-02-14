if !x
^^ Style/NegatedIf: Favor `unless` over `if` for negative conditions.
  do_something
end

if !condition
^^ Style/NegatedIf: Favor `unless` over `if` for negative conditions.
  foo
end

if !finished?
^^ Style/NegatedIf: Favor `unless` over `if` for negative conditions.
  retry
end
