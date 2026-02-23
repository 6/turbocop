self.ignored_columns += [:one, :two]
self.ignored_columns += [:three]
self.ignored_columns += %w[removed_column]
self.table_name = 'users'
self.primary_key = 'uuid'
has_many :posts
