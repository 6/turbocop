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

->(arg) { }

test do |key, value|
  puts something(binding)
end

1.times do |index; x|
  x = 10
  puts index
end

hash.each do |key, value|
  key, value = value, key
end

-> (_foo, bar) { puts bar }
