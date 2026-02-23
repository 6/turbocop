foo.each { |item| puts item }
bar.map { |x| x.to_s }
[1, 2].select { |n| n > 0 }
foo.each { |element| process(element) }
bar.map(&:to_s)
