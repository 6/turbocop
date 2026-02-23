add_column :users, :save, :string
                   ^^^^^ Rails/DangerousColumnNames: Avoid dangerous column names.
add_column :users, :class, :string
                   ^^^^^^ Rails/DangerousColumnNames: Avoid dangerous column names.
add_column :users, :id, :integer
                   ^^^ Rails/DangerousColumnNames: Avoid dangerous column names.
t.string :errors
         ^^^^^^^ Rails/DangerousColumnNames: Avoid dangerous column names.
t.integer :destroy
          ^^^^^^^^ Rails/DangerousColumnNames: Avoid dangerous column names.
