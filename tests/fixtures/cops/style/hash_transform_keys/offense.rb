x.each_with_object({}) { |(k, v), h| h[foo(k)] = v }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/HashTransformKeys: Prefer `transform_keys` over `each_with_object`.

x.each_with_object({}) { |(k, v), h| h[k.to_sym] = v }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/HashTransformKeys: Prefer `transform_keys` over `each_with_object`.

x.each_with_object({}) { |(k, v), h| h[k.to_s] = v }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/HashTransformKeys: Prefer `transform_keys` over `each_with_object`.
