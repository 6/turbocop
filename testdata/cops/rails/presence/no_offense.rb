a.presence
a.presence || b
a.present? ? b : a
a.blank? ? a : b
a.present? ? other_value : nil
x = y.present? ? z : nil

# elsif nodes should not be flagged
if x.present?
  x
elsif y.present?
  y
else
  z
end

# else branch containing a ternary (if node) should not be flagged on the outer if
if current.present?
  current
else
  something ? x : y
end

# chain with index access should not be flagged
a.present? ? a[1] : nil
a.present? ? a > 1 : nil
a <= 0 if a.present?

# chain patterns (rubocop-rails 2.34+, not yet supported)
field.destroy if field.present?
notification_subscription.destroy! if notification_subscription.present?
topic.update_pinned(false) if topic.present?
reply_to_post.present? ? reply_to_post.post_number : nil
email.present? ? email.downcase : nil
