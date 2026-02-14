create_table :users do |t|
^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/SchemaComment: Add a comment to the table for documentation.
  t.string :name
end

create_table :orders do |t|
^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/SchemaComment: Add a comment to the table for documentation.
  t.integer :user_id
  t.decimal :total
end

create_table :products, force: true do |t|
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/SchemaComment: Add a comment to the table for documentation.
  t.string :title
end
