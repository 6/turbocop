add_column :users, :active, :boolean
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/ThreeStateBooleanColumn: Boolean columns should always have a default value and a `NOT NULL` constraint.
t.boolean :active
^^^^^^^^^^^^^^^^^ Rails/ThreeStateBooleanColumn: Boolean columns should always have a default value and a `NOT NULL` constraint.
add_column :posts, :published, :boolean
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/ThreeStateBooleanColumn: Boolean columns should always have a default value and a `NOT NULL` constraint.
add_column :users, :admin, :boolean, default: false
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/ThreeStateBooleanColumn: Boolean columns should always have a default value and a `NOT NULL` constraint.
t.boolean :active, null: false
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/ThreeStateBooleanColumn: Boolean columns should always have a default value and a `NOT NULL` constraint.
