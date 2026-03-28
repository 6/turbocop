# each_with_object patterns
x.each_with_object({}) { |(k, v), h| h[k] = foo(v) }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/HashTransformValues: Prefer `transform_values` over `each_with_object`.

x.each_with_object({}) { |(k, v), h| h[k] = v.to_s }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/HashTransformValues: Prefer `transform_values` over `each_with_object`.

x.each_with_object({}) { |(k, v), h| h[k] = v.to_i }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/HashTransformValues: Prefer `transform_values` over `each_with_object`.

x.each_with_object({}) do |(k, v), h|
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/HashTransformValues: Prefer `transform_values` over `each_with_object`.
  h[k] = v * 2
end

# Hash[_.map {...}] pattern
Hash[x.map { |k, v| [k, foo(v)] }]
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/HashTransformValues: Prefer `transform_values` over `Hash[_.map {...}]`.

Hash[x.collect { |k, v| [k, v.to_s] }]
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/HashTransformValues: Prefer `transform_values` over `Hash[_.map {...}]`.

# _.map {...}.to_h pattern
x.map { |k, v| [k, v.to_s] }.to_h
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/HashTransformValues: Prefer `transform_values` over `map {...}.to_h`.

x.collect { |k, v| [k, v.to_i] }.to_h
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/HashTransformValues: Prefer `transform_values` over `map {...}.to_h`.

# _.to_h {...} pattern
x.to_h { |k, v| [k, v.to_s] }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/HashTransformValues: Prefer `transform_values` over `to_h {...}`.

x.to_h { |k, v| [k, foo(v)] }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/HashTransformValues: Prefer `transform_values` over `to_h {...}`.

# ::Hash[_.map {...}] with qualified constant path
::Hash[x.map { |k, v| [k, foo(v)] }]
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/HashTransformValues: Prefer `transform_values` over `Hash[_.map {...}]`.

# Multi-line map { }.to_h
x.map { |label, klass|
^ Style/HashTransformValues: Prefer `transform_values` over `map {...}.to_h`.
  [label, klass.to_s]
}.to_h

# map do...end.to_h
x.map do |name, attr|
^ Style/HashTransformValues: Prefer `transform_values` over `map {...}.to_h`.
  [name, attr.to_s]
end.to_h

# Hash[_.map do...end]
Hash[x.map do |name, members|
^ Style/HashTransformValues: Prefer `transform_values` over `Hash[_.map {...}]`.
  [name, members.sort]
end]

# ::Hash[_.map { }] multi-line
::Hash[raw_fonts.map { |label, font|
^ Style/HashTransformValues: Prefer `transform_values` over `Hash[_.map {...}]`.
  [label, font.to_s]
}]

# map{}.to_h where value contains key name as a symbol (not a variable reference)
x.map { |label, klass| [label, klass.map(&:label)] }.to_h
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/HashTransformValues: Prefer `transform_values` over `map {...}.to_h`.

# map do...end.to_h where value contains key name as a keyword argument
x.map do |name, attr|
^ Style/HashTransformValues: Prefer `transform_values` over `map {...}.to_h`.
  [name, Param.new(type: attr.type, name: nil)]
end.to_h

# Hash[_.map do...end] where value contains key name as a symbol key
Hash[x.map do |name, members|
^ Style/HashTransformValues: Prefer `transform_values` over `Hash[_.map {...}]`.
  [name, members.sort_by { |m| m[:name] }]
end]

# each_with_object where value contains key name as a symbol
x.each_with_object({}) do |(name, v), h|
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/HashTransformValues: Prefer `transform_values` over `each_with_object`.
  h[name] = v.merge(name: true)
end
