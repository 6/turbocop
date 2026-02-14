scope :active, -> { where(active: true) }
scope :recent, lambda { where("created_at > ?", 1.week.ago) }
scope :published, proc { where(published: true) }
scope :featured
scope :draft, -> { where(published: false) }
