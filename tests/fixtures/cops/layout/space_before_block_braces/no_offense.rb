[1, 2].each { |x| puts x }
[1, 2].map { |x| x * 2 }
foo.select { |x| x > 1 }
[1, 2].each do |x|
  puts x
end
x = {a: 1}
config.filter_sensitive_data('<SECRET>') 							{ ENV['SECRET'] }
items.each	{ |x| puts x }
-> { puts "hello" }
->(x) { x * 2 }
->() { 1 + 2 }
expect(-> { raise "boom" }).to raise_error
