add_column :users, :active, :boolean, null: false, default: false
t.boolean :active, null: false, default: false
add_column :users, :name, :string
t.string :name
t.boolean :enabled, null: false, default: false
add_column :posts, :visible, :boolean, default: true, null: false
