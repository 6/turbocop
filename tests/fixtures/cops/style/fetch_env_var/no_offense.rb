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
