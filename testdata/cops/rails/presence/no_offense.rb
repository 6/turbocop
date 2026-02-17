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
