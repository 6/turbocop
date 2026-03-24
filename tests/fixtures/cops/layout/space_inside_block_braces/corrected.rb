[1, 2].each { |x| puts x }
[1, 2].map { |x| x * 2 }
foo.select { |x| x > 1 }
escape_html = ->(str) { str.gsub("&", "&amp;") }
has_many :versions, -> { order("id ASC") }, class_name: "Foo"
action = -> { puts "hello" }
