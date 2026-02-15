add_column :table, :column, :integer, index: true
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/AddColumnIndex: `add_column` does not accept an `index` key, use `add_index` instead.
add_column :users, :group_id, :integer, index: true
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/AddColumnIndex: `add_column` does not accept an `index` key, use `add_index` instead.
add_column :posts, :category_id, :bigint, null: false, index: { unique: true }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/AddColumnIndex: `add_column` does not accept an `index` key, use `add_index` instead.
