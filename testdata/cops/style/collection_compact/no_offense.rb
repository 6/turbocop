array.compact
array.compact!
array.reject { |e| e.empty? }
array.reject { |e| e.zero? }
hash.reject { |k, v| v.blank? }
x = [1, nil, 2].compact
