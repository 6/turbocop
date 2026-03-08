Hash.new({key: :value})
Hash.new
Hash.new(capacity: 42)
Hash.new { |h, k| h[k] = [] }
x = Hash.new(0)

# Namespaced Hash classes should not be flagged
HashWithDotAccess::Hash.new(foo: "bar")
Hamster::Hash.new(key: 1, other: 2)
Configoro::Hash.new('host' => 'myhost', 'port' => 123)
Deprecation::Hash.new :class => 'pagination', :previous_label => '<<'
