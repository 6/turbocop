# Single-line braces — correct
items.each { |x| puts x }

# Multi-line do..end — correct
items.map do |x|
  x * 2
end

[1, 2].each { |i| i + 1 }

items.select do |x|
  x > 0
end

3.times { |i| puts i }

# AllowedMethods: lambda (default)
scope :paginate, lambda { |limit, max_id = nil|
  query = order(arel_table[:id].desc).limit(limit)
  query = query.where(arel_table[:id].lt(max_id)) if max_id.present?
  query
}

# AllowedMethods: proc (default)
handler = proc { |x|
  x * 2
}

# AllowedMethods: it (default)
it { is_expected.to eq(true) }
it {
  is_expected.to eq(true)
}

# Non-parenthesized argument block — ignored (changing delimiters changes binding)
expect { subject }.to change {
  redis.zrange(key, 0, -1)
}.from([]).to(["foo"])

# Non-parenthesized keyword hash value containing a block
get '/:path', to: redirect { |params|
  "/#{params[:path]}"
}
