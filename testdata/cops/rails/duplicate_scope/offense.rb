class Post < ApplicationRecord
  scope :published, -> { where(published: true) }
  scope :published, -> { where(status: "published") }
  ^^^^^ Rails/DuplicateScope: Duplicate scope `published` detected.
end
