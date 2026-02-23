class AddNameToUsers < ActiveRecord::Migration[7.0]
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/ReversibleMigrationMethodDefinition: Define both `up` and `down` methods, or use `change` for reversible migrations.
  def up
    add_column :users, :name, :string
  end
end

class RemoveOldColumn < ActiveRecord::Migration[7.0]
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/ReversibleMigrationMethodDefinition: Define both `up` and `down` methods, or use `change` for reversible migrations.
  def down
    add_column :posts, :legacy, :string
  end
end

class AddIndexToOrders < ActiveRecord::Migration[7.0]
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/ReversibleMigrationMethodDefinition: Define both `up` and `down` methods, or use `change` for reversible migrations.
  def up
    add_index :orders, :status
  end
end
