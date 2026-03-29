42 == x
^^^^^^^ Style/YodaCondition: Prefer non-Yoda conditions.

"hello" != y
^^^^^^^^^^^^ Style/YodaCondition: Prefer non-Yoda conditions.

nil == obj
^^^^^^^^^^ Style/YodaCondition: Prefer non-Yoda conditions.

true == flag
^^^^^^^^^^^^ Style/YodaCondition: Prefer non-Yoda conditions.

:foo == bar
^^^^^^^^^^^ Style/YodaCondition: Prefer non-Yoda conditions.

false != done
^^^^^^^^^^^^^ Style/YodaCondition: Prefer non-Yoda conditions.

CONST == value
^^^^^^^^^^^^^^ Style/YodaCondition: Prefer non-Yoda conditions.

MAX_SIZE >= count
^^^^^^^^^^^^^^^^^ Style/YodaCondition: Prefer non-Yoda conditions.

Config::LIMIT != total
^^^^^^^^^^^^^^^^^^^^^^ Style/YodaCondition: Prefer non-Yoda conditions.

`cmd` == value
^^^^^^^^^^^^^^ Style/YodaCondition: Prefer non-Yoda conditions.

:"#{name}=" == method_name
^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/YodaCondition: Prefer non-Yoda conditions.

/pattern/ == text
^^^^^^^^^^^^^^^^^ Style/YodaCondition: Prefer non-Yoda conditions.

{"foo" => ["bar"], "baz" => ["quux"]} == params
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/YodaCondition: Prefer non-Yoda conditions.

if [query] == found
   ^^^^^^^^^^^^^^^^ Style/YodaCondition: Prefer non-Yoda conditions.

if braid_spec.nil? || __FILE__ != braid_spec.gem_dir + '/lib/braid/check_gem.rb'
                      ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/YodaCondition: Prefer non-Yoda conditions.

[mailer.to_s, method.to_s, "deliver_now"] == job_args &&
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/YodaCondition: Prefer non-Yoda conditions.

raise ArgumentError, "bad currency pair" if [query[:base]] == query[:symbols]
                                            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/YodaCondition: Prefer non-Yoda conditions.

[username, password] == auth
^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/YodaCondition: Prefer non-Yoda conditions.

if [event.button, event.event_type] == target_info and
   ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/YodaCondition: Prefer non-Yoda conditions.

[mailer.to_s, method.to_s, "deliver_now"] == job_args &&
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/YodaCondition: Prefer non-Yoda conditions.

[username, password] == Redmon.config.secure.split(':')
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/YodaCondition: Prefer non-Yoda conditions.
