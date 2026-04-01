foo.each_key { |k| p k }
foo.each_value { |v| p v }
keys.each { |k| p k }
values.each { |v| p v }
foo.each { |k, v| do_something(k, v) }
{}.each_key { |k| p k }
# Both args used
foo.each { |k, v| puts "#{k}: #{v}" }
# Both args unused (skip)
foo.each { |_k, _v| puts "hello" }
# Single arg
foo.each { |item| p item }
# .each with arguments should not trigger (not a hash each pattern)
Resque::Failure.each(0, Resque::Failure.count, queue) do |_, item|
  puts item
end
collection.each(limit) { |_key, val| process(val) }
# keys.each / values.each with &block (non-symbol block_pass) should not trigger
packages.values.each(&blk)
@scopes.values.each(&block)
@namespaces.values.each(&block)
@cog_registry.cogs.keys.each(&method(:bind_cog))
# hash mutation: keys.each { |k| hash[k] = ... } should not trigger
hash.keys.each { |k| hash[k] = transform(hash[k]) }
params.keys.each { |key| params[key] = params[key].to_s }
rsp.keys.each { |k| rsp[k] = rsp[k].first }
settings.keys.each do |key|
  next unless value = settings[key]
  settings[key] = value.split
end
# _-prefixed param that IS actually used in the body should not trigger
data.each do |method_name, _apipie_dsl_data|
  description = define(method_name, _apipie_dsl_data)
end
# array-converter chains should not trigger
property_observer_list.to_a.each { |obs, opt| obs.call(self) }
packets.sort.each do |packet_name, packet_json|
  result << JSON.parse(packet_json)
end
# keys.each / values.each only trigger when the block is attached to `each` itself
gc_stat.keys.each.with_index { |k, i| puts k, i }
return vertices.values.each unless block_given?
