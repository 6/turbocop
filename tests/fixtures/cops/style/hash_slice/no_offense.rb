hash.slice(:foo, :bar)
hash.select { |k, v| v > 0 }
hash.select { |k, v| k == 0.0 }
hash.select { |k, v| do_something(k) }
hash.select
client_data[:headers]&.filter { |key, _value| cached_methods_params&.include?(key) }
coords.select { |x, y| (-y..y).include?(x) }
