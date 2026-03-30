{foo: 1, bar: 2, baz: 3}.except(:bar)
{foo: 1, bar: 2, baz: 3}.reject { |k, v| k != :bar }
{foo: 1, bar: 2, baz: 3}.select { |k, v| k == :bar }
{foo: 1, bar: 2, baz: 3}.reject { |k, v| v.eql? :bar }
{foo: 1, bar: 2, baz: 3}.reject
hash.reject { |k, v| k == 0.0 }
hash.reject { |k, v| ![:foo, :bar].include?(k) }
hash.select { |k, v| [:foo, :bar].include?(k) }
hash.reject { |k, v| ![1, 2].include?(v) }
hash.reject { |k, v| k.include?('oo') }
hash.reject { |k, v| (1..5).include?(k) }
