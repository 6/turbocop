class ExampleMigration < ActiveRecord::Migration[7.0]
  def change
    execute "ALTER TABLE pages ADD UNIQUE idx (page_id)"
    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/ReversibleMigration: execute is not reversible.
  end
end

class DropMigration < ActiveRecord::Migration[7.0]
  def change
    drop_table :users
    ^^^^^^^^^^^^^^^^^ Rails/ReversibleMigration: drop_table(without block) is not reversible.
  end
end

class RemoveMigration < ActiveRecord::Migration[7.0]
  def change
    remove_column(:suppliers, :qualification)
    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/ReversibleMigration: remove_column(without type) is not reversible.
  end
end

class ChangeColumnMigration < ActiveRecord::Migration[7.0]
  def change
    change_column(:posts, :state, :string)
    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/ReversibleMigration: change_column is not reversible.
  end
end
