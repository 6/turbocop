belongs_to :blog, optional: true
belongs_to :author, optional: false
belongs_to :category
belongs_to :user, class_name: 'Account'
has_many :posts
has_one :profile
