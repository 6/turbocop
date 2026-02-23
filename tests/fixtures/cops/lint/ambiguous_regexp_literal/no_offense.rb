p(/pattern/)
p(/pattern/, foo)
p(/pattern/.do_something)
assert(/some pattern/ =~ some_string)
foo = /pattern/
# %r{} syntax is never ambiguous (can't be confused with division)
assert_match %r{child-src 'self'}, @policy.build
assert_no_match %r{child-src}, @policy.build
param.split %r/=(.+)?/
argument_error.message =~ %r{undefined class/module ([\w:]*\w)}
assert_redirected_to %r(^http://test.host/route_two)
# Operator calls with regexp argument are never ambiguous
key =~ /\(\d+[if]?\)\z/
raw_host_with_port =~ /:(\d+)$/
get_header("CONTENT_TYPE") =~ /^([^,;]*)/
pim =~ /^visit_(.*)$/
