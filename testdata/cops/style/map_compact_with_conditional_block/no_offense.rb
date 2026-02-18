ary.filter_map { |x| x if x > 1 }
ary.map { |x| x.to_s }.compact
ary.map(&:to_s).compact
ary.compact
ary.filter_map { |item| item if item.valid? }
