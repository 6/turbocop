[1, 2, 3].each_with_object(0) { |x, sum| sum + x }
                           ^ Lint/EachWithObjectArgument: `each_with_object` called with an immutable argument.