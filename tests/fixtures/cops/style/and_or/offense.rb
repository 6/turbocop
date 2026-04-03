if a and b
     ^^^ Style/AndOr: Use `&&` instead of `and`.
  do_something
end

if a or b
     ^^ Style/AndOr: Use `||` instead of `or`.
  do_something
end

while x and y
        ^^^ Style/AndOr: Use `&&` instead of `and`.
  do_something
end

# FN fix: and/or inside parentheses within conditions
until (x or y)
         ^^ Style/AndOr: Use `||` instead of `or`.
  do_something
end

if (a and b)
      ^^^ Style/AndOr: Use `&&` instead of `and`.
  do_something
end

do_something unless (a or b)
                       ^^ Style/AndOr: Use `||` instead of `or`.

until (x or y or z)
         ^^ Style/AndOr: Use `||` instead of `or`.
              ^^ Style/AndOr: Use `||` instead of `or`.
  do_something
end

if foo and (bar or baz)
       ^^^ Style/AndOr: Use `&&` instead of `and`.
                ^^ Style/AndOr: Use `||` instead of `or`.
  do_something
end

if (a and b) or (c and d)
      ^^^ Style/AndOr: Use `&&` instead of `and`.
             ^^ Style/AndOr: Use `||` instead of `or`.
                   ^^^ Style/AndOr: Use `&&` instead of `and`.
  do_something
end

if arg.inject(true){|bool,item| bool and (item.is_a?(Integer) or item.is_a?(Range))}
                                     ^^^ Style/AndOr: Use `&&` instead of `and`.
                                                              ^^ Style/AndOr: Use `||` instead of `or`.
  do_something
end

return value if not (value.is_a?(Hash) and obj.is_a?(Hash))
                                       ^^^ Style/AndOr: Use `&&` instead of `and`.

if not (obj[key].nil? or obj[key].empty?)
                      ^^ Style/AndOr: Use `||` instead of `or`.
  do_something
end

if not (all_states or styles[STATES].nil? or styles[STATES].empty?)
                   ^^ Style/AndOr: Use `||` instead of `or`.
                                          ^^ Style/AndOr: Use `||` instead of `or`.
  do_something
end

styles = styles.merge(state) if not (state.nil? or state.empty?)
                                                ^^ Style/AndOr: Use `||` instead of `or`.

next if @seen[(mod.forge_name or mod.name)]
                              ^^ Style/AndOr: Use `||` instead of `or`.

unless suffix.split('.').all?{|s| s.empty? or Zonify::LDH_RE.match(s) }
                                           ^^ Style/AndOr: Use `||` instead of `or`.
  do_something
end

if (cookies[:user_lat] and cookies[:user_lon]).nil?
                       ^^^ Style/AndOr: Use `&&` instead of `and`.
  do_something
end
