add_column :users, :name, :string, null: false
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/NotNullColumn: Do not add a NOT NULL column without a default value.

add_column :posts, :title, :string, null: false
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/NotNullColumn: Do not add a NOT NULL column without a default value.

add_column :orders, :status, :integer, null: false
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/NotNullColumn: Do not add a NOT NULL column without a default value.
