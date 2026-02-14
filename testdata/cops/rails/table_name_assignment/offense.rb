class User < ActiveRecord::Base
  self.table_name = "legacy_users"
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/TableNameAssignment: Do not set `self.table_name`. Use conventions or rename the table.
end
