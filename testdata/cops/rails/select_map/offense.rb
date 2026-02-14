users.select { |u| u.active? }.map { |u| u.name }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/SelectMap: Use `filter_map` instead of `select.map`.

orders.select { |o| o.paid? }.map { |o| o.total }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/SelectMap: Use `filter_map` instead of `select.map`.

items.select { |i| i.valid? }.map { |i| i.to_s }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/SelectMap: Use `filter_map` instead of `select.map`.