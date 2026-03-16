users.index_by(&:id)
users.map { |u| u.name }.uniq
{}.to_h
users.each_with_object({}) { |u, h| h[u.id] = u.name }
users.group_by(&:role)
# Block body is not [key, element] pattern
users.map { |u| [u.id, u.name] }.to_h
items.map { |i| [i.key, transform(i)] }.to_h
data.map { |d| { d.id => d } }.to_h
# Identity mapping — key is element itself, not a method on it
records.each_with_object({}) { |record, h| h[record] = record }
Hash[columns.map { |name| [name, name] }]
# Numbered params where values are transformed — not index_by
x.map { [_1.to_sym, foo(_1)] }.to_h
x.to_h { [_2.to_sym, _1] }
Hash[x.map { [_1.to_sym, _2] }]
