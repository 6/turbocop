items.each do |item|
  next if item > 5
  puts item
end

[1, 2, 3].map { |x| x * 2 }

items.select { |item| item.valid? }

items.each { |item| break if item.nil? }

def foo
  return 42
end

# return with a value is allowed (per RuboCop)
items.each do |item|
  return item if item > 5
end

items.map do |x|
  return x * 2
end

# Block without arguments - not flagged
items.each do
  return
end

# Block without method chain - not flagged
each do |item|
  return
end

# define_method - return creates its own scope
define_method(:foo) do |arg|
  return
end

# lambda - return creates its own scope
items.each do |item|
  -> { return }
end
