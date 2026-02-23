x.transform_keys { |k| foo(k) }

x.each_with_object({}) { |(k, v), h| h[k] = v }

x.each_with_object({}) { |(k, v), h| h[k.to_sym] = foo(v) }

x.transform_keys(&:to_sym)

y = x.map { |k, v| [k.to_s, v] }.to_h

# Non-destructured params — not iterating a hash, so transform_keys doesn't apply
Base.classes.each_with_object({}) { |klass, classes| classes[klass.type] = klass }

# Hash inversion — value in output is the original key, not the original value
# This is NOT transform_keys since both key and value change
table.each_with_object({}) { |(id, attrs), index| index[attrs[:code]] = id }

# Another inversion pattern — assigning the key to a derived new key
data.each_with_object({}) { |(name, info), result| result[info[:label]] = name }
