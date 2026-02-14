class User < ApplicationRecord
  validates_presence_of :name
  ^^^^^^^^^^^^^^^^^^^^^ Rails/Validation: Use `validates :attr, presence: true` instead of `validates_presence_of`.
  validates_uniqueness_of :email
  ^^^^^^^^^^^^^^^^^^^^^^^ Rails/Validation: Use `validates :attr, uniqueness: true` instead of `validates_uniqueness_of`.
end
