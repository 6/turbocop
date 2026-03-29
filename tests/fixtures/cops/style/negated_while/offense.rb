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

while (not items.empty?)
^^^^^ Style/NegatedWhile: Favor `until` over `while` for negative conditions.
  items.shift
end

while(!workers.empty?)
^^^^^ Style/NegatedWhile: Favor `until` over `while` for negative conditions.
  workers.pop
end

while (!done?)
^^^^^ Style/NegatedWhile: Favor `until` over `while` for negative conditions.
  process
end

until !File.exist?(path)
^^^^^ Style/NegatedWhile: Favor `while` over `until` for negative conditions.
  path = next_path
end

x += 1 until !list.include?(x)
^^^^^^ Style/NegatedWhile: Favor `while` over `until` for negative conditions.

while (`curl -k -I https://localhost:8140/packages/ 2>/dev/null | grep "200 OK" > /dev/null`; ! $?.success?) do
^^^^^ Style/NegatedWhile: Favor `until` over `while` for negative conditions.
  sleep 10
end
