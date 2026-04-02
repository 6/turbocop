ENV.fetch('X')
ENV.fetch('X', default_value)
env_hash['X']
config['X']
something['X']
CONFIG['DB']
!ENV['X']
ENV['X'].some_method
ENV['X']&.some_method
ENV['X'] == 1
ENV['X'] != 1
ENV['X'] ||= y
ENV['X'] &&= y
ENV['X'] || default_value
ENV['X'] || ENV['Y'] || default_value
if ENV['X']
  do_something
end
unless ENV['X']
  do_something
end
do_something if ENV['X']
do_something unless ENV['X']
value = ENV['X'] ? 'a' : 'b'
ENV['X'] || ENV.fetch('Y', nil)
# Comparison method as argument: 1 == ENV['X']
1 == ENV['X']
1 != ENV['X']
# Body-in-condition: ENV['X'] in body when same key in condition
if ENV['X']
  puts ENV['X']
end
if ENV['X'].present?
  config = ENV['X']
end
do_something(ENV['X']) if ENV['X'].present?
ENV['X'].empty? ? "" : YAML.parse(ENV['X']).to_ruby
# ENV['KEY'] guarded by ENV.has_key?('KEY') in condition
if ENV.has_key?('KEY')
  puts ENV['KEY']
end
config = ENV['KEY'] if ENV.has_key?('KEY')
# ENV['KEY'] guarded by ENV.key?('KEY') in condition
if ENV.key?('KEY')
  puts ENV['KEY']
end
config = ENV['KEY'] if ENV.key?('KEY')
# ENV['KEY'] guarded by ENV.include?('KEY') in condition
if ENV.include?('KEY')
  puts ENV['KEY']
end
config = ENV['KEY'] if ENV.include?('KEY')
# unless with ENV.key? guard
unless ENV.key?('KEY')
  fallback
else
  puts ENV['KEY']
end
# ::ENV (fully qualified) is not matched by RuboCop
::ENV['X']
::ENV["Y"]
x = ::ENV['Z']
# === is a comparison method
allowed === ENV['DATABASE_URL']
# Quote mismatch: condition uses double quotes, body uses single quotes
if ENV["KEY"]
  puts ENV['KEY']
end
# Guard with different quote style
if ENV.key?("KEY")
  puts ENV['KEY']
end
# Condition with ENV['X'] == comparison, body uses same key with different quotes
if ENV["X"] == foo
  puts ENV['X']
end
# ENV['X'].in? predicate method in condition
if ENV["X"].in?(%w[A B C])
  puts ENV["X"]
end
# %w[...].include?(ENV['X']) in condition
if %w[A B C].include?(ENV["X"])
  puts ENV["X"]
end
# ENV in condition with && (2 elements) — ENV is direct child, body suppressed
if ENV['X'] && other
  puts ENV['X']
end
# ENV['X'] == value condition with body usage
if ENV['X'] == 'production'
  puts ENV['X']
end
if ENV["X"] != 'test'
  puts ENV["X"]
end
# ENV['X'] in && condition: same key nested inside method call is suppressed
# by structural equality (child_nodes.any? matches the direct child ENV['X'])
if ENV['X'] and hash(ENV['X']) != service_hash
  set_hash = hash(ENV['X'])
end
# Assignment in condition without parens: RuboCop treats ENV as flag (child_nodes match)
if var = ENV['X']
  puts var
end
# elsif with assignment
if true
  puts "yes"
elsif var = ENV['X']
  puts var
end
# Parenthesized bare ENV condition is still a flag
if(ENV['MODEL'])
  puts ENV['MODEL']
end
# Reverse regex match with ENV on the argument side is treated as a flag
if /1|true/ =~ ENV['LISTEN_GEM_SIMULATE_FSEVENT']
  puts ENV['LISTEN_GEM_SIMULATE_FSEVENT']
end
# Duplicate ENV key on both sides of || is accepted by RuboCop
config.api_key = ENV['BUGSNAG_API_KEY'] || ENV['BUGSNAG_API_KEY']
# Same-key ENV ||= ENV is accepted
ENV['OPENAI_API_KEY'] ||= ENV['OPENAI_API_KEY']
# Non-local assignment conditions still suppress a bare direct ENV child
if @@bin = ENV['DIFFY_DIFF']
  puts @@bin
end
