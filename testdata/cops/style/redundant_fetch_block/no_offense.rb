hash.fetch(:key, 5)
hash.fetch(:key, :value)
hash.fetch(:key) { computed_value }
hash.fetch(:key) { |k| process(k) }
hash.fetch(:key, 'default')
hash[:key]
