# create_table without comment
create_table :users do |t|
^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/SchemaComment: New database table without `comment`.
  t.string :name
end

create_table :orders do |t|
^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/SchemaComment: New database table without `comment`.
  t.integer :user_id
  t.decimal :total
end

create_table :products, force: true do |t|
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/SchemaComment: New database table without `comment`.
  t.string :title
end

# create_table with nil comment
create_table :items, comment: nil do |t|
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/SchemaComment: New database table without `comment`.
  t.string :name
end

# create_table with empty comment
create_table :categories, comment: '' do |t|
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/SchemaComment: New database table without `comment`.
  t.string :name
end

# add_column without comment
add_column :users, :name, :string
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/SchemaComment: New database column without `comment`.

add_column :users, :age, :integer, default: 0
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/SchemaComment: New database column without `comment`.

# add_column with nil comment
add_column :users, :role, :string, comment: nil
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/SchemaComment: New database column without `comment`.

# add_column with empty comment
add_column :users, :status, :string, comment: ''
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/SchemaComment: New database column without `comment`.

# column methods inside create_table block without comment (table HAS comment)
create_table :posts, comment: 'Posts table' do |t|
  t.string :title
  ^^^^^^^^^^^^^^^^ Rails/SchemaComment: New database column without `comment`.
  t.integer :views
  ^^^^^^^^^^^^^^^^^ Rails/SchemaComment: New database column without `comment`.
  t.column :body, :text
  ^^^^^^^^^^^^^^^^^^^^^ Rails/SchemaComment: New database column without `comment`.
end

# t.references without comment (table HAS comment)
create_table :comments, comment: 'Comments' do |t|
  t.references :user
  ^^^^^^^^^^^^^^^^^^ Rails/SchemaComment: New database column without `comment`.
end

# t.belongs_to without comment (table HAS comment)
create_table :tags, comment: 'Tags' do |t|
  t.belongs_to :post
  ^^^^^^^^^^^^^^^^^^ Rails/SchemaComment: New database column without `comment`.
end
