unless !x
^^^^^^ Style/NegatedUnless: Favor `if` over `unless` for negative conditions.
  do_something
end

unless !condition
^^^^^^ Style/NegatedUnless: Favor `if` over `unless` for negative conditions.
  foo
end

unless !finished?
^^^^^^ Style/NegatedUnless: Favor `if` over `unless` for negative conditions.
  retry
end
