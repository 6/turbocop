arr.compact_blank
arr.reject { |x| x.nil? }
arr.select { |x| x.size > 1 }
arr.reject { |x| x.empty? }
collection.compact_blank!
collection.select { |e| e.blank? }
collection.reject! { |e| e.blank? }

# Block parameter doesn't match receiver
arr.reject { |x| y.blank? }
arr.select { |item| other.present? }

# Mismatched hash args: first arg used, not second
arr.reject { |a, b| a.blank? }
