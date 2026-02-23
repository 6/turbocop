hash.slice(:foo, :bar)
hash.select { |k, v| v > 0 }
hash.select { |k, v| k == 0.0 }
hash.select { |k, v| do_something(k) }
hash.select
