if a && b
  do_something
end

if a || b
  do_something
end

while x && y
  do_something
end

# FN fix: and/or inside parentheses within conditions
until (x || y)
  do_something
end

if (a && b)
  do_something
end

do_something unless (a || b)

until (x || y || z)
  do_something
end

if foo && (bar || baz)
  do_something
end

if (a && b) || (c && d)
  do_something
end

if arg.inject(true){|bool,item| bool && (item.is_a?(Integer) || item.is_a?(Range))}
  do_something
end

return value if not (value.is_a?(Hash) && obj.is_a?(Hash))

if not (obj[key].nil? || obj[key].empty?)
  do_something
end

if not (all_states || styles[STATES].nil? || styles[STATES].empty?)
  do_something
end

styles = styles.merge(state) if not (state.nil? || state.empty?)

next if @seen[(mod.forge_name || mod.name)]

unless suffix.split('.').all?{|s| s.empty? || Zonify::LDH_RE.match(s) }
  do_something
end

if (cookies[:user_lat] && cookies[:user_lon]).nil?
  do_something
end
