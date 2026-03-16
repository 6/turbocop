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

# Numbered parameters (_1) — Ruby 2.7+
x.map { [_1.to_sym, _1] }.to_h
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/IndexBy: Use `index_by` instead of `map { ... }.to_h`.

x.to_h { [_1.to_sym, _1] }
^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/IndexBy: Use `index_by` instead of `to_h { ... }`.

Hash[x.map { [_1.to_sym, _1] }]
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/IndexBy: Use `index_by` instead of `Hash[map { ... }]`.
