foo.to_h { |x| [x, x * 2] }
foo.map { |x| [x, x * 2] }
foo.map { |x| x.to_s }.to_set
foo.each_with_object({}) { |(k, v), h| h[k] = v }
hash.to_h
foo.map(&:to_s).to_a
