[1, 2, 3].map { |el| [el, foo(el)] }.to_h
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/IndexWith: Use `index_with` instead of `map { ... }.to_h`.
[1, 2, 3].each_with_object({}) { |el, h| h[el] = foo(el) }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/IndexWith: Use `index_with` instead of `each_with_object`.
[1, 2, 3].to_h { |el| [el, foo(el)] }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/IndexWith: Use `index_with` instead of `to_h { ... }`.
