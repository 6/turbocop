class User < ApplicationRecord
  validates :name, presence: true, allow_nil: true
  ^^^^^^^^^ Rails/RedundantAllowNil: Remove redundant `allow_nil` when `presence` validation is also specified.
end

class Post < ApplicationRecord
  validates :title, allow_nil: true, presence: true
  ^^^^^^^^^ Rails/RedundantAllowNil: Remove redundant `allow_nil` when `presence` validation is also specified.
end

class Comment < ApplicationRecord
  validates :body, presence: true, allow_nil: true, length: { maximum: 500 }
  ^^^^^^^^^ Rails/RedundantAllowNil: Remove redundant `allow_nil` when `presence` validation is also specified.
end
