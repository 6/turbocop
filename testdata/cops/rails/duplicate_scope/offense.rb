class Post < ApplicationRecord
  scope :published, -> { where(published: true) }
  scope :published, -> { where(status: "published") }
  ^^^^^ Rails/DuplicateScope: Duplicate scope `published` detected.
end

class User < ApplicationRecord
  scope :active, -> { where(active: true) }
  scope :active, -> { where(status: "active") }
  ^^^^^ Rails/DuplicateScope: Duplicate scope `active` detected.
end

class Order < ApplicationRecord
  scope :recent, -> { order(created_at: :desc) }
  scope :pending, -> { where(status: "pending") }
  scope :recent, -> { order(updated_at: :desc) }
  ^^^^^ Rails/DuplicateScope: Duplicate scope `recent` detected.
end
