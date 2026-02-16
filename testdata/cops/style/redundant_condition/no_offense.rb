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
