add_column :users, :active, :boolean, null: false, default: false
t.boolean :active, null: false, default: false
add_column :users, :name, :string
t.string :name
t.boolean :enabled, null: false, default: false
add_column :posts, :visible, :boolean, default: true, null: false

# Migration with change_column_null (add_column form)
class AddFoo < ActiveRecord::Migration[7.0]
  def change
    add_column :users, :foo, :boolean
    change_column_null :users, :foo, false
  end
end

# Migration with change_column_null (create_table form)
class CreatePosts < ActiveRecord::Migration[7.0]
  def change
    create_table :posts do |t|
      t.boolean :active
      t.string :title
    end
    change_column_null :posts, :active, false
  end
end

# Migration with change_column_null (change_table t.column form)
class UpdateUsers < ActiveRecord::Migration[7.0]
  def change
    change_table :users do |t|
      t.column :verified, :boolean
    end
    change_column_null :users, :verified, false
  end
end
