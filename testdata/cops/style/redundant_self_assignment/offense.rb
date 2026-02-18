arr = arr.sort!
^^^^^^^^^^^^^^^ Style/RedundantSelfAssignment: Redundant self-assignment. `sort!` modifies `arr` in place.

str = str.gsub!('a', 'b')
^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/RedundantSelfAssignment: Redundant self-assignment. `gsub!` modifies `str` in place.

hash = hash.merge!(other)
^^^^^^^^^^^^^^^^^^^^^^^^^ Style/RedundantSelfAssignment: Redundant self-assignment. `merge!` modifies `hash` in place.
