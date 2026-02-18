foo.map(&:to_s)
bar.select(&:valid?)
items.reject(&:nil?)
foo.map { |x| x.to_s(16) }
bar.each { |x| puts x }
baz.map { |x, y| x + y }
