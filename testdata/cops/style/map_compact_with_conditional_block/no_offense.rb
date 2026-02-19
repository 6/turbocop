ary.filter_map { |x| x if x > 1 }
ary.map { |x| x.to_s }.compact
ary.map(&:to_s).compact
ary.compact
ary.filter_map { |item| item if item.valid? }
# Truthy branch returns a transformed value, not the block parameter — not replaceable with select/reject
ary.map { |c| Regexp.last_match(1) if c =~ /pattern/ }.compact
ary.map { |x| x.upcase if x.valid? }.compact
ary.map { |item| transform(item) if item.present? }.compact
