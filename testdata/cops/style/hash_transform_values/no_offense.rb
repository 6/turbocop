x.transform_values { |v| foo(v) }

x.each_with_object({}) { |(k, v), h| h[k] = v }

x.each_with_object({}) { |(k, v), h| h[k.to_sym] = foo(v) }

x.transform_values(&:to_s)

y = x.map { |k, v| [k, v.to_s] }.to_h

# Value expression references the key variable — not a transform_values candidate
group_columns.each_with_object({}) do |(aliaz, col_name), types|
  types[aliaz] = col_name.try(:type_caster) || fetch(aliaz)
end
