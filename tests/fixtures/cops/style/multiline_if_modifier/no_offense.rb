run if cond

run unless cond

run if cond &&
       cond2

run unless cond &&
           cond2

if cond
  something
end

unless cond
  something
end

# Backslash continuation makes it a single logical line
raise ArgumentError, "missing discovery method" \
  unless opts.has_key?('discovery')

raise ArgumentError, "invalid method" \
  if opts['method'] != 'base'

do_something(arg1, arg2) \
  unless condition && other_condition
