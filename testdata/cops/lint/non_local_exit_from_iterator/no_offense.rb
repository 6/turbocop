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
