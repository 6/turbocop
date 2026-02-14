has_many :items
has_many :items, dependent: :destroy
belongs_to :user
has_one :profile
has_and_belongs_to_many :tags
