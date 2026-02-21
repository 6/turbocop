hash[:a] = 1
hash.merge!(a: 1, b: 2, c: 3)
hash.merge(a: 1)
hash.merge!
hash.merge!(other_hash)
# Non-pure receiver with multiple pairs — not flagged
obj.options.merge!(a: 1, b: 2)
hash[key].merge!(a: 1, b: 2)
Foo::Bar.defaults.merge!(x: 1, y: 2)
# merge! as last expression in a block — return value used
{ key: "value" }.tap do |h|
  h.merge!(extra: true)
end
items.each do |item|
  item.data.merge!(status: :done)
end
# merge! inside single-line .each block — return value unused
items.each { |item| item.merge!(key: value) }
# merge! with splat
hash.merge!(**other)
# Multi-line merge! as last expression in method — return value used
def liquid_locals
  super.merge!({
                 custom_message: @custom_message
               })
end
# merge! before rescue — return value used as last expression before rescue
begin
  h.merge!(a: 1)
rescue StandardError => e
  handle(e)
end
