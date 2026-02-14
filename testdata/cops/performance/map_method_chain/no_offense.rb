x.map { |e| e.foo.bar }
x.map(&:foo).select(&:bar)
x.map(&:foo)
arr.map(&:foo).flat_map(&:bar)
arr.select(&:valid?).map(&:name)
