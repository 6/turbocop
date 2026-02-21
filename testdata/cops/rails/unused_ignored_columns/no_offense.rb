class User < ApplicationRecord
  # Column exists in schema
  self.ignored_columns = [:account]
end

class User < ApplicationRecord
  # Column exists (string form)
  self.ignored_columns = ['name']
end

class User < ApplicationRecord
  # Non-literal value — skip
  self.ignored_columns = array
end

module Abc
  # Not a class — skip
  self.ignored_columns = [:real_name]
end
