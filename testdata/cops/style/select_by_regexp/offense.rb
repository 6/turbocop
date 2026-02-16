array.select { |x| x.match?(/regexp/) }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/SelectByRegexp: Prefer `grep` to `select` with a regexp match.

array.select { |x| /regexp/.match?(x) }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/SelectByRegexp: Prefer `grep` to `select` with a regexp match.

array.reject { |x| x =~ /regexp/ }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/SelectByRegexp: Prefer `grep_v` to `reject` with a regexp match.
