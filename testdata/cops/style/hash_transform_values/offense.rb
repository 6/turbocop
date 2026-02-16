x.each_with_object({}) { |(k, v), h| h[k] = foo(v) }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/HashTransformValues: Prefer `transform_values` over `each_with_object`.

x.each_with_object({}) { |(k, v), h| h[k] = v.to_s }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/HashTransformValues: Prefer `transform_values` over `each_with_object`.

x.each_with_object({}) { |(k, v), h| h[k] = v.to_i }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/HashTransformValues: Prefer `transform_values` over `each_with_object`.
