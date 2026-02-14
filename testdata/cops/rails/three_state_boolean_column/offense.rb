add_column :users, :active, :boolean
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/ThreeStateBooleanColumn: Add `null: false` to boolean columns to avoid three-state booleans.
t.boolean :active
^^^^^^^^^^^^^^^^^ Rails/ThreeStateBooleanColumn: Add `null: false` to boolean columns to avoid three-state booleans.
add_column :posts, :published, :boolean
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/ThreeStateBooleanColumn: Add `null: false` to boolean columns to avoid three-state booleans.
