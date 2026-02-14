scope :active, -> { where(active: true) }
scope :recent, lambda { where("created_at > ?", 1.week.ago) }
