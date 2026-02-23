x.map { |e| e.foo.bar }
x.map(&:foo).select(&:bar)
x.map(&:foo)
arr.map(&:foo).flat_map(&:bar)
arr.select(&:valid?).map(&:name)
# Chained map with blocks, not symbol args — not flagged
arr.map { |x| x.split('=', 2) }.map { |k, v| [k.downcase, v] }
