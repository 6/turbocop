class AddNameToUsers < ActiveRecord::Migration[7.0]
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/ReversibleMigrationMethodDefinition: Define both `up` and `down` methods, or use `change` for reversible migrations.
  def up
    add_column :users, :name, :string
  end
end
