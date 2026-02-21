class User < ApplicationRecord
  self.ignored_columns = [:real_name]
                          ^^^^^^^^^^ Rails/UnusedIgnoredColumns: Remove `real_name` from `ignored_columns` because the column does not exist.
end

class User < ApplicationRecord
  self.ignored_columns = ['real_name']
                          ^^^^^^^^^^^ Rails/UnusedIgnoredColumns: Remove `real_name` from `ignored_columns` because the column does not exist.
end

class User < ApplicationRecord
  self.ignored_columns = [:real_name, :nickname]
                          ^^^^^^^^^^ Rails/UnusedIgnoredColumns: Remove `real_name` from `ignored_columns` because the column does not exist.
                                      ^^^^^^^^^ Rails/UnusedIgnoredColumns: Remove `nickname` from `ignored_columns` because the column does not exist.
end
