class User < ApplicationRecord
  self.ignored_columns = [:account]
end

class User < ApplicationRecord
  self.ignored_columns = ['account']
end

class User < ApplicationRecord
  self.ignored_columns = array
end

module Abc
  self.ignored_columns = [:real_name]
end
