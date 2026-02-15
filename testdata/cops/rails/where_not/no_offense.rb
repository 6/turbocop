User.where.not(status: "active")
User.where(status: "active")
User.where("name = ?", "foo")
User.where.not(id: [1, 2])
Post.where(published: true)
# Complex SQL with NOT should not be flagged
where('NOT EXISTS (SELECT * FROM statuses_tags forbidden WHERE forbidden.status_id = statuses.id)')
where('scheduled_at IS NOT NULL AND scheduled_at <= ?', Time.now.utc)
where('name NOT LIKE ?', '%test%')
