users.map { |u| [u.id, u] }.to_h
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/IndexBy: Use `index_by` instead of `map { ... }.to_h`.

posts.collect { |p| [p.slug, p] }.to_h
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/IndexBy: Use `index_by` instead of `map { ... }.to_h`.

items.to_h { |item| [item.name, item] }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/IndexBy: Use `index_by` instead of `to_h { ... }`.

data.each_with_object({}) { |el, acc| acc[el.key] = el }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/IndexBy: Use `index_by` instead of `each_with_object`.

Hash[fields.map { |f| [f.name, f] }]
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/IndexBy: Use `index_by` instead of `Hash[map { ... }]`.
