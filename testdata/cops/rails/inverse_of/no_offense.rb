class User < ApplicationRecord
  has_many :posts
  has_many :posts, foreign_key: :author_id, inverse_of: :author
  has_one :profile, as: :profilable, inverse_of: :user
  belongs_to :company
  has_many :followers, -> { order(:name) }, through: :relationships
  belongs_to :imageable, polymorphic: true
  has_many :active_accounts, -> { merge(Account.active) }, through: :memberships, source: :account
end
