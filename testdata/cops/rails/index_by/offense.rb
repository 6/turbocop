users.map { |u| [u.id, u] }.to_h
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/IndexBy: Use `index_by` instead of `map { ... }.to_h`.

posts.map { |p| [p.slug, p] }.to_h
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/IndexBy: Use `index_by` instead of `map { ... }.to_h`.

items.map { |item| [item.name, item] }.to_h
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/IndexBy: Use `index_by` instead of `map { ... }.to_h`.
