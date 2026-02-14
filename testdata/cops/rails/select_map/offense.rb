users.select { |u| u.active? }.map { |u| u.name }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/SelectMap: Use `filter_map` instead of `select.map`.