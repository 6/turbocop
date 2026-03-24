puts"hello"
    ^ Layout/SpaceBeforeFirstArg: Put one space between the method name and the first argument.
puts"hello", "world"
    ^ Layout/SpaceBeforeFirstArg: Put one space between the method name and the first argument.
foo"bar"
   ^ Layout/SpaceBeforeFirstArg: Put one space between the method name and the first argument.

# Extra spaces without alignment on adjacent lines are offenses
something  x
         ^^ Layout/SpaceBeforeFirstArg: Put one space between the method name and the first argument.
something   y, z
         ^^^ Layout/SpaceBeforeFirstArg: Put one space between the method name and the first argument.

# Extra space with receiver
a.something  y, z
           ^^ Layout/SpaceBeforeFirstArg: Put one space between the method name and the first argument.

# Extra space with safe navigation
a&.something  y, z
            ^^ Layout/SpaceBeforeFirstArg: Put one space between the method name and the first argument.

# Extra spaces not aligned with anything on adjacent lines
describe  "with http basic auth features" do
        ^^ Layout/SpaceBeforeFirstArg: Put one space between the method name and the first argument.
end

# has_many/belongs_to with extra spaces not aligned
has_many   :security_groups
        ^^^ Layout/SpaceBeforeFirstArg: Put one space between the method name and the first argument.

# Vertical argument position NOT aligned (no_space case)
obj = a_method(arg, arg2)
obj.no_parenthesized'asdf'
                    ^ Layout/SpaceBeforeFirstArg: Put one space between the method name and the first argument.
