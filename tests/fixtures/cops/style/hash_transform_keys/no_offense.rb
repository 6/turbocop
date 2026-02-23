x.transform_keys { |k| foo(k) }

x.each_with_object({}) { |(k, v), h| h[k] = v }

x.each_with_object({}) { |(k, v), h| h[k.to_sym] = foo(v) }

x.transform_keys(&:to_sym)

y = x.map { |k, v| [k.to_s, v] }.to_h

# Non-destructured params — not iterating a hash, so transform_keys doesn't apply
Base.classes.each_with_object({}) { |klass, classes| classes[klass.type] = klass }
