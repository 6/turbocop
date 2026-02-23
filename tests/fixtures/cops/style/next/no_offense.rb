[].each do |o|
  next unless o == 1
  puts o
end
[].each do |o|
  if o == 1
    puts o
  else
    puts "other"
  end
end
[].each { |a| return 'yes' if a == 1 }
items.map do |item|
  if item
    process(item)
  end
end
