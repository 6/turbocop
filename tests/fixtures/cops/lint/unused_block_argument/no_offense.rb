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

# Nested block shadows outer param, but outer is also used directly
items.each do |item|
  puts item
  results.map do |item|
    item.name
  end
end

# Outer param used before nested block that shadows it
data.each do |value|
  puts value
  items.map do |value|
    value.to_s
  end
end

# Nested block does NOT shadow - different param name, both used
items.each do |item|
  results.map do |result|
    [item, result]
  end
end

# Operator-assign counts as a read (x += 1 means x = x + 1)
counters.each do |key, value|
  value += 1
  puts key
end

# Or-assign counts as a read (x ||= val means x = x || val)
items.each do |item|
  item ||= default_item
end

# And-assign counts as a read (x &&= val means x = x && val)
records.each do |record|
  record &&= nil
end

# def on block arg receiver counts as a use (singleton method definition)
Object.new.tap do |o|
  def o.to_str
    "hello"
  end
end

# Block arg used inside def as receiver
described_class.create(site).tap do |c|
  def c.request_url_once(url)
    Net::HTTPResponse.new('1.1', '200', 'okay')
  end
end

# Block inside def method with param used
def process
  items.each do |item|
    puts item
  end
end

# Block inside class > def with param used
class Worker
  def run
    tasks.each do |task|
      task.execute
    end
  end
end
