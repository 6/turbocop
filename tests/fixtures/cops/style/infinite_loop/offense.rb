while true
^^^^^ Style/InfiniteLoop: Use `Kernel#loop` for infinite loops.
  work
end

until false
^^^^^ Style/InfiniteLoop: Use `Kernel#loop` for infinite loops.
  work
end

while true
^^^^^ Style/InfiniteLoop: Use `Kernel#loop` for infinite loops.
  break if done?
end
