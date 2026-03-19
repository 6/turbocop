# create_table with comment (columns not checked since table has comment
# and all columns also have comments)
create_table :users, comment: 'Stores user accounts' do |t|
  t.string :name, comment: 'Full name'
end

create_table :posts, comment: 'Blog posts' do |t|
  t.string :title, comment: 'Post title'
end

# add_column with comment
add_column :users, :name, :string, comment: 'Full name'

add_column :users, :age, :integer, null: false, comment: 'Age in years', default: 0

# column methods with comment inside create_table block
create_table :orders, comment: 'Customer orders' do |t|
  t.string :number, comment: 'Order number'
  t.integer :total, comment: 'Total in cents'
  t.column :status, :string, comment: 'Order status'
  t.references :user, comment: 'Associated user'
  t.belongs_to :store, comment: 'Associated store'
end

# comment is a local variable
create_table :invoices, comment: 'Invoices' do |t|
  desc = 'A description'
  t.string :number, comment: desc
end

# Sequel ORM — add_column with only 2 args (no keyword hash).
# parser_arg_count = 2, below the 3-4 range for add_column pattern.
Sequel.migration do
  alter_table(:users) do
    add_column :name, String
    add_column :age, Integer
  end
end

# Sequel change block — 2-arg add_column
Sequel.migration do
  change do
    alter_table(:records) do
      add_column :payload, :text
    end
  end
end

