do_something do |used, _unused|
  puts used
end

do_something do
  puts :foo
end

[1, 2, 3].each do |x|
  puts x
end

do_something { |unused| }

items.map do |item|
  item.name
end
