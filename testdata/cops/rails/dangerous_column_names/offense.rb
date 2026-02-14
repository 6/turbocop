add_column :users, :type, :string
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/DangerousColumnNames: Avoid using `type` as a column name. It conflicts with ActiveRecord internals.
add_column :users, :class, :string
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/DangerousColumnNames: Avoid using `class` as a column name. It conflicts with ActiveRecord internals.
add_column :users, :id, :integer
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/DangerousColumnNames: Avoid using `id` as a column name. It conflicts with ActiveRecord internals.
