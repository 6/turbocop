scope :active, -> { where(active: true) }
scope :recent, lambda { where("created_at > ?", 1.week.ago) }
scope :published, proc { where(published: true) }
scope :featured
scope :draft, -> { where(published: false) }
# Routing scope DSL is not a model scope
scope "/customize", constraints: AdminConstraint.new do
end
scope :visible_groups, Proc.new { |user| user.groups }
