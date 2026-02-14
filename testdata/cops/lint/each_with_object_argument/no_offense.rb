[1, 2, 3].each_with_object([]) { |x, acc| acc << x }
[1, 2, 3].each_with_object({}) { |x, acc| acc[x] = true }