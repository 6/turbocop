arr.compact_blank
arr.reject { |x| x.nil? }
arr.select { |x| x.size > 1 }
arr.reject { |x| x.empty? }
collection.compact_blank!
collection.select { |e| e.blank? }
collection.reject! { |e| e.blank? }
