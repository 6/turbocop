class User < ActiveRecord::Base
  self.table_name = "legacy_users"
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/TableNameAssignment: Do not set `self.table_name`. Use conventions or rename the table.
end

class Post < ApplicationRecord
  self.table_name = "articles"
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/TableNameAssignment: Do not set `self.table_name`. Use conventions or rename the table.
end

class Comment < ApplicationRecord
  self.table_name = "feedback"
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/TableNameAssignment: Do not set `self.table_name`. Use conventions or rename the table.
end
