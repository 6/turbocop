foo.to_h { |x| [x, x * 2] }
foo.map { |x| [x, x * 2] }
foo.map { |x| x.to_s }.to_set
foo.each_with_object({}) { |(k, v), h| h[k] = v }
hash.to_h
foo.map(&:to_s).to_a
enum.map(&blk).to_h
return map(&block).to_h if block_given?
return map(&block).to_h(*args) if block_given?
