x.transform_values { |v| foo(v) }

x.each_with_object({}) { |(k, v), h| h[k] = v }

x.each_with_object({}) { |(k, v), h| h[k.to_sym] = foo(v) }

x.transform_values(&:to_s)

y = x.map { |k, v| [k, v.to_s] }.to_h
