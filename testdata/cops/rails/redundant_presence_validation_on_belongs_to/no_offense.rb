class Post < ApplicationRecord
  belongs_to :user
  validates :title, presence: true
  validates :body, length: { minimum: 10 }
  validates :status, inclusion: { in: %w[draft published] }
end
