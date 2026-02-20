# turbocop-filename: db/migrate/001_example.rb
def change
  change_table :users, bulk: true do |t|
    t.string :name, null: false
    t.string :address, null: true
  end
end

def change
  change_table :users do |t|
    t.string :name, null: false
  end
end

def change
  add_column :users, :name, :string, null: false
end

def change
  add_reference :users, :team
  add_column :teams, :name, :string, null: false
  remove_column :posts, :owner_name
end
