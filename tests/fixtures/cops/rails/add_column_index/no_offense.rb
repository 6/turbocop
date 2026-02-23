add_column :users, :name, :string
add_column :users, :email, :string
add_column :users, :age, :integer
add_column :posts, :title, :string
add_column :table, :column, :integer, default: 0
add_column :posts, :user_id, :integer
add_index :posts, :user_id
