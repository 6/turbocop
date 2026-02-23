class Post < ApplicationRecord
  scope :visible, -> { where(visible: true) }
  ^^^^^ Rails/DuplicateScope: Multiple scopes share this same expression.
  scope :shown, -> { where(visible: true) }
  ^^^^^ Rails/DuplicateScope: Multiple scopes share this same expression.
end

class User < ApplicationRecord
  scope :active, -> { where(active: true) }
  ^^^^^ Rails/DuplicateScope: Multiple scopes share this same expression.
  scope :enabled, -> { where(active: true) }
  ^^^^^ Rails/DuplicateScope: Multiple scopes share this same expression.
end

class Order < ApplicationRecord
  scope :recent, -> { order(created_at: :desc) }
  ^^^^^ Rails/DuplicateScope: Multiple scopes share this same expression.
  scope :pending, -> { where(status: "pending") }
  scope :newest, -> { order(created_at: :desc) }
  ^^^^^ Rails/DuplicateScope: Multiple scopes share this same expression.
end
