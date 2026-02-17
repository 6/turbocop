str.match?(/\A#{CONST}/)
           ^^^^^^^^^^^ Performance/ConstantRegexp: Extract this regexp into a constant, memoize it, or append an `/o` option to its options.
str.match?(/\A#{CONST1}something#{CONST2}\z/)
           ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/ConstantRegexp: Extract this regexp into a constant, memoize it, or append an `/o` option to its options.
str.match?(/\A#{CONST1}something#{Regexp.escape(CONST2)}\z/)
           ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/ConstantRegexp: Extract this regexp into a constant, memoize it, or append an `/o` option to its options.
