dest = src.map { |x| x * 2 }
items.map { |item| item.to_s }
src.each { |x| process(x) }
src.each { |x| puts x }
src.each { |x| x.save; log(x) }
src.each { |e| @dest << e }
self.each { |e| dest << e }
each { |e| dest << e }
CSV.generate do |csv|
  items.each { |item| csv << [item.name] }
end
items.each { |item| output << item.to_s }
dest = "hello"
src.each { |e| dest << e.to_s }
# Variable used between init and each - not a pure map pattern
attributes = []
attributes << "Name: #{record.name}"
attributes << "Email: #{record.email}"
records.each do |record|
  attributes << "#{record.key}: #{record.value}"
end
# Variable used inside an intermediate assignment's value expression
order = []
entries = src.map do |entry|
  order << entry.full_name
  transform(entry)
end
entries.each do |entry|
  order << entry.path
end
# Safe navigation on receiver - not flagged
results = []
collection&.each do |item|
  results << item.to_s
end
# Operator assignment (+=) between init and each
attachments = []
attachments += existing_items(list)
items.each do |item|
  attachments << { data: item.data, name: item.name }
end
# Or-assignment (||=) between init and each
values = []
values ||= defaults
src.each { |e| values << e }
# binding inside each block captures destination variable implicitly
linespecs = []
acl.grants.each do |grant|
  linespecs.push(ERB.new(template, trim_mode: '-').result(binding))
end
# Qualified constant Array.new (e.g. Machinery::Array.new) is not an empty array init
packages = Machinery::Array.new
src.each { |e| packages << e.to_s }
# Destination variable referenced in the pushed expression
new_nodeset = []
ordered_nodeset.each do |node|
  new_nodeset << XPathNode.new(node, position: new_nodeset.size + 1)
end
# Destination variable referenced as keyword argument in push expression
routes = []
last_run.each do |train, data|
  routes << Route.new(train, routes: routes)
end
# [].tap with multiple statements in tap block (not pure map)
[].tap do |items|
  setup_context
  src.each { |e| items << e }
end
