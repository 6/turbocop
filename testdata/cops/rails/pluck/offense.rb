users.map { |u| u[:name] }
^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/Pluck: Use `pluck(:key)` instead of `map { |item| item[:key] }`.

posts.map { |p| p[:title] }
^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/Pluck: Use `pluck(:key)` instead of `map { |item| item[:key] }`.

items.map { |item| item[:price] }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/Pluck: Use `pluck(:key)` instead of `map { |item| item[:key] }`.
