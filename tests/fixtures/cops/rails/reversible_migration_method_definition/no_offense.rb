class AddNameToUsers < ActiveRecord::Migration[7.0]
  def change
    add_column :users, :name, :string
  end
end

class RemoveNameFromUsers < ActiveRecord::Migration[7.0]
  def up
    remove_column :users, :name
  end

  def down
    add_column :users, :name, :string
  end
end
