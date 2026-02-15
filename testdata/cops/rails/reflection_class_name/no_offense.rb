has_many :items, class_name: "Item"
has_many :items
has_many :items, dependent: :destroy
belongs_to :user
belongs_to :user, class_name: "SpecialUser"
has_one :profile
has_one :profile, class_name: "UserProfile"
has_and_belongs_to_many :tags
has_and_belongs_to_many :tags, class_name: "CustomTag"
