x.each_with_object({}) { |(k, v), h| h[foo(k)] = v }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/HashTransformKeys: Prefer `transform_keys` over `each_with_object`.

x.each_with_object({}) { |(k, v), h| h[k.to_sym] = v }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/HashTransformKeys: Prefer `transform_keys` over `each_with_object`.

x.each_with_object({}) { |(k, v), h| h[k.to_s] = v }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/HashTransformKeys: Prefer `transform_keys` over `each_with_object`.

@attrs = HashWithIndifferentAccess.new(Hash[attrs.map { |k, v| [ to_key(k), v ] }])
                                       ^^^^ Style/HashTransformKeys: Prefer `transform_keys` over `Hash[_.map {...}]`.

query_hash = Hash[options.map { |k, v| [service_key_mappings[k], v] }]
             ^^^^ Style/HashTransformKeys: Prefer `transform_keys` over `Hash[_.map {...}]`.

query_hash = Hash[options.map { |k, v| [ACCOUNT_KEY_MAPPINGS[k], v] }]
             ^^^^ Style/HashTransformKeys: Prefer `transform_keys` over `Hash[_.map {...}]`.

attributes = Hash[attributes.map { |k, v| [k.to_s, v] }]
             ^^^^ Style/HashTransformKeys: Prefer `transform_keys` over `Hash[_.map {...}]`.

restrictions = Hash[restrictions.map { |k, v| [k.to_sym, v] }]
               ^^^^ Style/HashTransformKeys: Prefer `transform_keys` over `Hash[_.map {...}]`.

Hash[test_app_hosts_by_spec.map do |spec, value|
^^^^ Style/HashTransformKeys: Prefer `transform_keys` over `Hash[_.map {...}]`.
  [spec.name, value]
end]

Hash[result.map { |k, v| [prefix + k, v] }]
^^^^ Style/HashTransformKeys: Prefer `transform_keys` over `Hash[_.map {...}]`.

Hash[options.map { |k, v| [k.to_sym, v] }]
^^^^ Style/HashTransformKeys: Prefer `transform_keys` over `Hash[_.map {...}]`.

::Hash[options.map { |k, v| [k.to_sym, v] }]
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/HashTransformKeys: Prefer `transform_keys` over `Hash[_.map {...}]`.
