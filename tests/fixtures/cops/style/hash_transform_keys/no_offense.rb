x.transform_keys { |k| foo(k) }

x.each_with_object({}) { |(k, v), h| h[k] = v }

x.each_with_object({}) { |(k, v), h| h[k.to_sym] = foo(v) }

x.transform_keys(&:to_sym)

# Non-destructured params — not iterating a hash, so transform_keys doesn't apply
Base.classes.each_with_object({}) { |klass, classes| classes[klass.type] = klass }

# Hash inversion — value in output is the original key, not the original value
# This is NOT transform_keys since both key and value change
table.each_with_object({}) { |(id, attrs), index| index[attrs[:code]] = id }

# Another inversion pattern — assigning the key to a derived new key
data.each_with_object({}) { |(name, info), result| result[info[:label]] = name }

# Key expression derives from the value param, not the key — not a key transformation
Hash[pod_target_installation_results.map do |_, result|
  [result.native_target, result]
end]

# Array-like receiver via `each_with_index` should not be treated as a hash
ordering = Hash[drilldown.select { |r| (r[0].to_s.length > 1) && (r[0][0] == r[0][-1]) }.each_with_index.map { |r, i| [r[0].delete('/'), i] }]

# New key derives from the value param, not the original key
FORMATS.each_with_object({}) { |(_name, format), hsh| hsh[format.media_type] = format }

# Destructured rest means this is not a simple two-element hash pair pattern
TABLES.each do |table_name, url|
  lines = URI.open(url).readlines(chomp: true)
  index = lines.grep_v(/^#|^\s*$/).map(&:split).each_with_object({}) { |(idx, value, *), hash| hash[idx.to_i] = value }
end
