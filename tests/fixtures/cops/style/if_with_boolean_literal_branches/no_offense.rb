if condition
  true
else
  false
end

unless condition
  false
else
  true
end

if foo == bar
  do_something
else
  false
end

if foo && bar
  true
else
  false
end

if foo || bar
  true
else
  false
end

# Ternary with non-boolean condition (hash lookup, variable, method returning non-boolean)
instance_options[:relationships].following[object.id] ? true : false
Redis::Alfred.set(key, value, nx: true, ex: timeout) ? true : false
app ? true : false

# Ternary with non-boolean method (no ? suffix)
foo.do_something ? true : false

# Regex match operators are not boolean (=~ returns MatchData or nil)
result =~ /^running/ ? true : false
text =~ /pattern/ ? true : false
line !~ /^#/ ? true : false
str =~ /mingw|win32|cygwin/ ? true : false
if text =~ /^\s*$/
  true
else
  false
end

# Spaceship operator does not return boolean
foo <=> bar ? true : false

# Multiple elsif with boolean literal branches - should NOT be flagged
if foo
  true
elsif bar > baz
  true
elsif qux > quux
  true
else
  false
end

# Multi-elsif chain (2 elsifs) with predicate methods
if !current_version_array.any?
  false
elsif !new_version_array.any?
  true
elsif have_any_matching_version?
  true
else
  false
end

# Multi-elsif chain (3 elsifs)
if template_name.blank?
  false
elsif template_options.empty?
  true
elsif template_options[:only] && template_options[:only].include?(action_name.to_sym)
  true
elsif template_options[:except] && !template_options[:except].include?(action_name.to_sym)
  true
else
  false
end
