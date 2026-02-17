scope :published, -> { where(hidden: false) }
scope :active, -> { where(active: true) }
scope :recent, -> { order(created_at: :desc) }
scope :visible, -> { where(visible: true) }
has_many :comments
belongs_to :author
