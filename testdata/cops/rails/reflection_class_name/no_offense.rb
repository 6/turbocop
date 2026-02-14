has_many :items
has_many :items, dependent: :destroy
belongs_to :user
