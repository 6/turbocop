x.map { |e| e.foo.bar }
x.map(&:foo).select(&:bar)
