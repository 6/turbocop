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

h[:constraints] = field.constraints.map { |k, v| [k.underscore, v] }.to_h
                  ^ Style/HashTransformKeys: Prefer `transform_keys` over `map {...}.to_h`.

string_tags = raw_tags.collect { |k, v| [k.to_s, v] }.to_h
              ^ Style/HashTransformKeys: Prefer `transform_keys` over `map {...}.to_h`.

string_tags = tags.collect { |k, v| [k.to_s, v] }.to_h
              ^ Style/HashTransformKeys: Prefer `transform_keys` over `map {...}.to_h`.

let(:rack_headers) { headers.map { |k, v| [RackSupport.header_to_rack(k), v] }.to_h }
                     ^ Style/HashTransformKeys: Prefer `transform_keys` over `map {...}.to_h`.

span.to_hash.map { |k, v| [k.to_s, v] }.to_h
^ Style/HashTransformKeys: Prefer `transform_keys` over `map {...}.to_h`.

span.to_hash.map { |k, v| [k.to_s, v] }.to_h
^ Style/HashTransformKeys: Prefer `transform_keys` over `map {...}.to_h`.

spectra.map do |filename, lines|
^ Style/HashTransformKeys: Prefer `transform_keys` over `map {...}.to_h`.
  [normalized_path(filename), lines]
end.to_h

hash.map { |key, value| [key.to_sym, value] }.to_h
^ Style/HashTransformKeys: Prefer `transform_keys` over `map {...}.to_h`.

x.to_h { |k, v| [k.to_sym, v] }
^ Style/HashTransformKeys: Prefer `transform_keys` over `to_h {...}`.
