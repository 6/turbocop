# rblint-filename: db/migrate/001_example.rb
def change
  change_table :users do |t|
  ^^^^^^^^^^^^^^^^^^^ Rails/BulkChangeTable: You can combine alter queries using `bulk: true` options.
    t.string :name, null: false
    t.string :address, null: true
  end
end

def change
  change_table :orders do |t|
  ^^^^^^^^^^^^^^^^^^^^ Rails/BulkChangeTable: You can combine alter queries using `bulk: true` options.
    t.index :name
    t.index :address
  end
end

def change
  add_column :users, :name, :string, null: false
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/BulkChangeTable: You can use `change_table :users, bulk: true` to combine alter queries.
  remove_column :users, :nickname
end
