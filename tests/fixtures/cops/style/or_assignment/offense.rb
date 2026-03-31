x = x ? x : 'fallback'
^^^^^^^^^^^^^^^^^^^^^^^ Style/OrAssignment: Use the double pipe equals operator `||=` instead.

output = path unless output
^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/OrAssignment: Use the double pipe equals operator `||=` instead.

unless @groups
^^^^^^^^^^^^^^ Style/OrAssignment: Use the double pipe equals operator `||=` instead.
  @groups = 'default'
end

content_type = 'application/json' unless content_type
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/OrAssignment: Use the double pipe equals operator `||=` instead.

sleep_time = 30000000 unless sleep_time
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/OrAssignment: Use the double pipe equals operator `||=` instead.

matched = nil
unless matched
^^^^^^^^^^^^^^ Style/OrAssignment: Use the double pipe equals operator `||=` instead.
  matched = available_servers.find { |s| s.name }
end

queue_id = subscribe_limits_events(100000) unless queue_id
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/OrAssignment: Use the double pipe equals operator `||=` instead.

@foo = 'default' unless @foo
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/OrAssignment: Use the double pipe equals operator `||=` instead.

@@foo = 'default' unless @@foo
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/OrAssignment: Use the double pipe equals operator `||=` instead.

$foo = 'default' unless $foo
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/OrAssignment: Use the double pipe equals operator `||=` instead.
