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

# Nested: argless block inside non-chained block - no offense
find_each do |item|
  item.with_lock do
    return if item.stock == 0
  end
end

# Nested: no-arg block wrapping non-chained block - no offense
transaction do
  return unless update_necessary?
  find_each do |item|
    return if item.stock == 0
    item.update!(foobar: true)
  end
end
