class User < ApplicationRecord
  validates :name, presence: true, allow_nil: true
  ^^^^^^^^^ Rails/RedundantAllowNil: Remove redundant `allow_nil` when `presence` validation is also specified.
end
