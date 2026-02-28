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
# merge! result used as argument to another method
get @action, params: @params.merge!(before: Time.current.to_i.to_s)
# merge! result used as method argument (positional)
do_something(hash.merge!(key: val))
# merge! result assigned to variable
result = hash.merge!(a: 1)
# merge! with conflict resolution block — not replaceable with []=
hash.merge!(a: 1) { |key, old, new| old }
hash.merge!(a: 1, b: 2) { |key, old, new| old }
values.merge!(max_age: max_age) { |_key, v1, v2| v1 || v2 }
