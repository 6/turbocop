Const&.do_something
     ^^ Lint/RedundantSafeNavigation: Redundant safe navigation detected, use `.` instead.

self&.foo
    ^^ Lint/RedundantSafeNavigation: Redundant safe navigation detected, use `.` instead.

foo.to_s&.strip
        ^^ Lint/RedundantSafeNavigation: Redundant safe navigation detected, use `.` instead.

42&.minutes
  ^^ Lint/RedundantSafeNavigation: Redundant safe navigation detected, use `.` instead.

'hello'&.upcase
       ^^ Lint/RedundantSafeNavigation: Redundant safe navigation detected, use `.` instead.

foo&.to_h || {}
   ^^^^^^^^^^^^ Lint/RedundantSafeNavigation: Redundant safe navigation with default literal detected.

foo&.to_a || []
   ^^^^^^^^^^^^ Lint/RedundantSafeNavigation: Redundant safe navigation with default literal detected.

foo&.to_i || 0
   ^^^^^^^^^^^ Lint/RedundantSafeNavigation: Redundant safe navigation with default literal detected.

foo&.to_f || 0.0
   ^^^^^^^^^^^^^ Lint/RedundantSafeNavigation: Redundant safe navigation with default literal detected.

foo&.to_s || ''
   ^^^^^^^^^^^^ Lint/RedundantSafeNavigation: Redundant safe navigation with default literal detected.

foo&.to_h { |k, v| [k, v] } || {}
   ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/RedundantSafeNavigation: Redundant safe navigation with default literal detected.

# Case 5: AllowedMethods in conditional context
if foo&.respond_to?(:bar)
      ^^ Lint/RedundantSafeNavigation: Redundant safe navigation detected, use `.` instead.
  do_something
elsif foo&.respond_to?(:baz)
         ^^ Lint/RedundantSafeNavigation: Redundant safe navigation detected, use `.` instead.
  do_something_else
end

do_something unless foo&.respond_to?(:bar)
                       ^^ Lint/RedundantSafeNavigation: Redundant safe navigation detected, use `.` instead.

while foo&.respond_to?(:bar)
         ^^ Lint/RedundantSafeNavigation: Redundant safe navigation detected, use `.` instead.
  do_something
end

begin
  do_something
end until foo&.respond_to?(:bar)
             ^^ Lint/RedundantSafeNavigation: Redundant safe navigation detected, use `.` instead.

do_something if foo&.respond_to?(:bar) && !foo&.respond_to?(:baz)
                   ^^ Lint/RedundantSafeNavigation: Redundant safe navigation detected, use `.` instead.
                                              ^^ Lint/RedundantSafeNavigation: Redundant safe navigation detected, use `.` instead.

if foo&.is_a?(String)
      ^^ Lint/RedundantSafeNavigation: Redundant safe navigation detected, use `.` instead.
  do_something
end

do_something if foo&.kind_of?(Hash)
                   ^^ Lint/RedundantSafeNavigation: Redundant safe navigation detected, use `.` instead.

return unless foo&.eql?('bar')
                 ^^ Lint/RedundantSafeNavigation: Redundant safe navigation detected, use `.` instead.

foo&.instance_of?(String) ? 'yes' : 'no'
   ^^ Lint/RedundantSafeNavigation: Redundant safe navigation detected, use `.` instead.

# AllowedMethods with || in condition
return unless options[:name] && options[:value]&.is_a?(Hash)
                                               ^^ Lint/RedundantSafeNavigation: Redundant safe navigation detected, use `.` instead.

# AllowedMethods with eql? in if condition
if parameters[:method]&.eql?('POST')
                      ^^ Lint/RedundantSafeNavigation: Redundant safe navigation detected, use `.` instead.
  handle_post
end

# equal? in condition
do_something if foo&.equal?(bar)
                   ^^ Lint/RedundantSafeNavigation: Redundant safe navigation detected, use `.` instead.

# AllowedMethods in standalone && / || expressions (not inside if/unless)
user&.is_a?(Admin) && user.respond_to?(:roles)
    ^^ Lint/RedundantSafeNavigation: Redundant safe navigation detected, use `.` instead.

options[:min]&.is_a?(Proc) && options[:max]&.is_a?(Proc)
             ^^ Lint/RedundantSafeNavigation: Redundant safe navigation detected, use `.` instead.
                                           ^^ Lint/RedundantSafeNavigation: Redundant safe navigation detected, use `.` instead.

condition&.respond_to?(:method) || fallback
         ^^ Lint/RedundantSafeNavigation: Redundant safe navigation detected, use `.` instead.

# Backtick literal receiver — always returns String, &. is redundant
`cat /tmp/pid`&.strip
              ^^ Lint/RedundantSafeNavigation: Redundant safe navigation detected, use `.` instead.

# Negation with ! wrapping AllowedMethod in || (standalone)
!charge&.is_a?(Klass) || fallback
       ^^ Lint/RedundantSafeNavigation: Redundant safe navigation detected, use `.` instead.

# respond_to? with string argument (not symbol) in &&
obj&.respond_to?('some_method?') && obj.some_method?
   ^^ Lint/RedundantSafeNavigation: Redundant safe navigation detected, use `.` instead.

# Negation with ! in standalone || expression
!item&.is_a?(Widget) || !(item&.active? || item&.pending?)
     ^^ Lint/RedundantSafeNavigation: Redundant safe navigation detected, use `.` instead.

# AllowedMethod in ternary condition (via &&)
obj&.is_a?(String) && obj.valid? ? 'yes' : 'no'
   ^^ Lint/RedundantSafeNavigation: Redundant safe navigation detected, use `.` instead.

# Chained calls ending with &.is_a? in || false
node.args&.last&.is_a?(TrueNode) || false
               ^^ Lint/RedundantSafeNavigation: Redundant safe navigation detected, use `.` instead.

# AllowedMethod in && inside parentheses inside ||
node.nil? || (node&.name&.to_s == "foo" && node.parent&.is_a?(SelfNode))
                                                      ^^ Lint/RedundantSafeNavigation: Redundant safe navigation detected, use `.` instead.

# if with parenthesized && inside condition
if persisted? && active? && (!obj&.is_a?(Klass) || !(obj&.running?))
                                 ^^ Lint/RedundantSafeNavigation: Redundant safe navigation detected, use `.` instead.
  do_something
end

# Assignment with ||= and parenthesized &&
zone ||= (user&.is_a?(Admin) && user.valid?) ? user.zone : default
              ^^ Lint/RedundantSafeNavigation: Redundant safe navigation detected, use `.` instead.

# AllowedMethod in && inside method body
def check_roles?(roles)
  user&.is_a?(Admin) && user.respond_to?(:roles)
      ^^ Lint/RedundantSafeNavigation: Redundant safe navigation detected, use `.` instead.
end

# rescue => self&.foo — self is never nil, &. is redundant
begin
  something
rescue => self&.captured_error
              ^^ Lint/RedundantSafeNavigation: Redundant safe navigation detected, use `.` instead.
  handle
end

# AllowedMethod in ternary condition nested under ||
( response&.result&.respond_to?( :error_description ) ? response.result.error_description : nil ) || ''
                  ^^ Lint/RedundantSafeNavigation: Redundant safe navigation detected, use `.` instead.

# AllowedMethod in ternary condition nested under ||
offset = seconds || (current_user&.respond_to?(:auto_timeout) ? current_user.auto_timeout : nil)
                                 ^^ Lint/RedundantSafeNavigation: Redundant safe navigation detected, use `.` instead.

# AllowedMethod in modifier if nested under ||
@current_ability || (scope.current_ability if scope&.respond_to?(:current_ability))
                                                   ^^ Lint/RedundantSafeNavigation: Redundant safe navigation detected, use `.` instead.

# AllowedMethod in modifier if nested under ||
@collection || (scope.context[:collection] if scope&.respond_to?(:context))
                                                   ^^ Lint/RedundantSafeNavigation: Redundant safe navigation detected, use `.` instead.

# AllowedMethod in modifier if nested under || with other safe-nav calls nearby
linked_to_studies = a&.studies.present? || (a&.study.present? if a&.respond_to?(:study))
                                                                  ^^ Lint/RedundantSafeNavigation: Redundant safe navigation detected, use `.` instead.
