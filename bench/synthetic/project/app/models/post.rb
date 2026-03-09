class Post < ApplicationRecord
  belongs_to :user

  scope :recent, -> { where("created_at >= ?", 1.week.ago) }
  scope :by_status, ->(status) { where("status >= ? AND status <= ?", status, status + 10) }

  def self.active_tags
    # CompactBlank
    Tag.all.reject { |t| t.blank? }
  end

  def self.tag_map
    # IndexWith: each_with_object form
    Tag.all.each_with_object({}) { |t, h| h[t] = t.name }
  end

  def self.first_title
    # Pick
    Post.pluck(:title).first
  end

  def self.all_names
    # Pluck
    Post.all.map { |p| p[:name] }
  end

  def self.without_comments
    # WhereMissing
    Post.left_joins(:comments).where(comments: { id: nil })
  end

  def formatted_date
    # ToFormattedS
    created_at.to_formatted_s(:short)
  end

  def date_string
    # ToSWithArgument
    created_at.to_s(:db)
  end

  def metadata
    # TopLevelHashWithIndifferentAccess
    HashWithIndifferentAccess.new(title: title)
  end

  def full_day_range
    # ExpandedDateRange
    created_at.beginning_of_day..created_at.end_of_day
  end
end
