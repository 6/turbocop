[1, 2].each { |x| puts x }
[1, 2].map { |x| x * 2 }
[1, 2].each do |x|
  puts x
end
foo.select { |x| x > 1 }
x = {}
items.each { |x|
  puts x
}
items.map {
  42
}
escape_html = ->(str) { str.gsub("&", "&amp;") }
has_many :versions, -> { order("id ASC") }, class_name: "Foo"
action = -> { puts "hello" }
f = ->(x) { x + 1 }
empty_lambda = ->(x) {}
empty_proc = proc {|x|
}
g = -> {
  42
}
