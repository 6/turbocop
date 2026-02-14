add_column :users, :active, :boolean, null: false
t.boolean :active, null: false
add_column :users, :name, :string
t.string :name
t.boolean :enabled, null: false, default: false
