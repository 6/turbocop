class Post < ApplicationRecord
  belongs_to :user
  validates :title, presence: true
  validates :body, length: { minimum: 10 }
  validates :status, inclusion: { in: %w[draft published] }
end

class MediaAttachment < ApplicationRecord
  belongs_to :account, optional: true
  validates :account, presence: true
end

# presence: false explicitly disables validation — should not be flagged
class Audit < ApplicationRecord
  belongs_to :auditable, polymorphic: true
  validates :auditable, presence: false
end
