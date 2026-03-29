x ? y : z

if a
  b
else
  c
end

x || y

a ? a.to_s : b

# if with elsif is not a redundant condition (can't simplify to ||)
if object
  object
elsif @template_object.instance_variable_defined?("@#{@object_name}")
  @template_object.instance_variable_get("@#{@object_name}")
end

# Multi-line else branch — vendor skips these
if options[:binding]
  options[:binding]
else
  default_host = environment == "development" ? "localhost" : "0.0.0.0"
  ENV.fetch("BINDING", default_host)
end

# predicate? ? true : value is only flagged when true branch is `true` and
# else branch is NOT `true` — here both branches are literals, not the pattern
x.nil? ? "yes" : "no"

# Non-predicate condition with true is not flagged
x ? true : y

# hash key assignment in else — vendor skips these (use_hash_key_assignment?)
if @cache[key]
  @cache[key]
else
  @cache[key] = heavy_load[key]
end

# ternary in else branch — vendor skips (use_if_branch?)
if @options[:id_param]
  @options[:id_param]
else
  parent? ? :"#{name}_id" : :id
end

# predicate with non-true branch is not flagged
if a.zero?
  false
else
  a
end

# predicate with number branch is not flagged
if a.zero?
  1
else
  a
end

# predicate with string branch
if a.zero?
  'true'
else
  a
end

# no-else branch, condition does NOT match true branch
if do_something
  something_else
end

# unless without else is not flagged
unless b
  y(x, z)
end

# unless where condition does not match else
unless a
  b
else
  c
end

# modifier if/unless not flagged
bar if bar
bar unless bar

# predicate with non-call condition (local var, ivar, etc)
variable = do_something
if variable
  true
else
  a
end

if @variable
  true
else
  a
end

# predicate+true where condition is not a method call (bracket access)
if a[:key]
  true
else
  a
end

# true branch is true but else branch is also true — not flagged
a.zero? ? true : true

# Assignment branches with DIFFERENT target variables — not flagged
if foo
  @foo = foo
else
  @baz = 'quux'
end

# Method branches with different receivers — not flagged
if x
  X.find(x)
else
  Y.find(y)
end

# hash key access in method branches — not flagged
if foo
  bar[foo]
else
  bar[1]
end

# predicate with no else body
if a.zero?
  true
else
end

# predicate with no true body
if a.zero?
else
  a
end

# ternary with different condition and branches
a.zero? ? a : b

# FP fix: predicate with block in ternary — not flagged (block changes AST type in RuboCop)
libs.all? { |lib| load_library(lib) } ? true : nil

# unless with condition in else branch but modifier-if fallback — skipped by use_if_branch?
unless layout_without_inheritance
  parent.layout if parent?
else
  layout_without_inheritance
end

# unless with predicate+true but multiline fallback body — RuboCop skips these
unless include_controls_list.empty?
  group_data[:controls].any? do |control_id|
    include_controls_list.any? { |id| id.match?(control_id) }
  end
else
  true
end
