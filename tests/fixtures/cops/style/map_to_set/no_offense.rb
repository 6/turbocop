foo.to_set { |x| x * 2 }
foo.map { |x| x.to_s }
foo.map { |x| x.to_s }.to_a
foo.each_with_object([]) { |x, a| a << x }
items.to_set
foo.map(&:to_s).to_h
