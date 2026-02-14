[1, 2, 3].filter_map { |x| x > 1 ? x * 2 : nil }
[1, 2, 3].select { |x| x > 1 }
[1, 2, 3].map { |x| x * 2 }