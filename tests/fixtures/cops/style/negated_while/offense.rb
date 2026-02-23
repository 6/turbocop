while !x
^^^^^ Style/NegatedWhile: Favor `until` over `while` for negative conditions.
  do_something
end

while !done?
^^^^^ Style/NegatedWhile: Favor `until` over `while` for negative conditions.
  process
end

while !queue.empty?
^^^^^ Style/NegatedWhile: Favor `until` over `while` for negative conditions.
  work
end
