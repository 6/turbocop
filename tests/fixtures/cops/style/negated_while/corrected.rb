until x
  do_something
end

until done?
  process
end

until queue.empty?
  work
end

until items.empty?
  items.shift
end

until workers.empty?
  workers.pop
end

until done?
  process
end

while File.exist?(path)
  path = next_path
end

x += 1 while list.include?(x)
