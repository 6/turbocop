dest = src.map { |x| x * 2 }
items.map { |item| item.to_s }
src.each { |x| process(x) }
src.each { |x| puts x }
src.each { |x| x.save; log(x) }
src.each { |e| @dest << e }
self.each { |e| dest << e }
each { |e| dest << e }
CSV.generate do |csv|
  items.each { |item| csv << [item.name] }
end
items.each { |item| output << item.to_s }
dest = "hello"
src.each { |e| dest << e.to_s }
