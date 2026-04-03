{foo: 1, bar: 2}.select { |k, v| k == :foo }
                 ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/HashSlice: Use `slice(:foo)` instead.
hash.select { |k, v| k == :name }
     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/HashSlice: Use `slice(:name)` instead.
hash.filter { |k, v| k == 'key' }
     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/HashSlice: Use `slice('key')` instead.
hash.select { |k, _| allowed_keys.include?(k) }
     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/HashSlice: Use `slice(*allowed_keys)` instead.

select { |k, _v| keys.include?(k) }
^ Style/HashSlice: Use `slice(*keys)` instead.

base_attrs = last_rec.attributes.reject { |k, _v| !base_cols.include?(k) }
                                 ^ Style/HashSlice: Use `slice(*base_cols)` instead.

self.select { |key, value| key.in? keys }
     ^ Style/HashSlice: Use `slice(*keys)` instead.

remapped_docs = doc["_source"]["doc_versions"].map{|version| version.reject{|key, value| !keys_to_keep.include?(key)}}    
                                                                     ^ Style/HashSlice: Use `slice(*keys_to_keep)` instead.

options[:generator].new(options.to_hash.reject {|k,v| !options[:generator].options.valid_keys.include?(k) })
                                        ^ Style/HashSlice: Use `slice(*options[:generator].options.valid_keys)` instead.

record.share_hash(opts).reject { |k, _| configuring_fields.exclude?(k) },
                        ^ Style/HashSlice: Use `slice(*configuring_fields)` instead.

uri.query_values =  uri.query_values.reject do |k,v|
                                     ^ Style/HashSlice: Use `slice(*allowed_params)` instead.
  !allowed_params.include? k
end

headers = headers.select { |header, value| header.in?(HEADERS) }
                  ^ Style/HashSlice: Use `slice(*HEADERS)` instead.
