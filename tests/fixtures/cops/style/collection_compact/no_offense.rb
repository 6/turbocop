array.compact
array.compact!
array.reject { |e| e.empty? }
array.reject { |e| e.zero? }
hash.reject { |k, v| v.blank? }
x = [1, nil, 2].compact
# Method chain on block param - not equivalent to .compact
items.reject { |item| item.target_status.nil? }
entries.reject { |e| e.value.nil? }
