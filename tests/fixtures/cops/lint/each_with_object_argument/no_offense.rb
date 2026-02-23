[1, 2, 3].each_with_object([]) { |x, acc| acc << x }
[1, 2, 3].each_with_object({}) { |x, acc| acc[x] = true }
[1, 2].each_with_object("") { |x, acc| acc << x.to_s }
[1, 2].map { |x| x * 2 }
items.each { |x| puts x }
