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

# Extra spaces with tabs in the gap should still be flagged
method		:arg
      ^^ Layout/SpaceBeforeFirstArg: Put one space between the method name and the first argument.

# The 2nd nearest non-blank line should not cause false alignment.
# RuboCop only checks the nearest non-blank line in pass 1 and the nearest
# same-indent line in pass 2. Here the nearest non-blank line (x :y) does NOT
# have a \s\S boundary at col 12, but the 2nd line (xxxxxxxxxxz :thing) does.
# Nitrocop should NOT use the 2nd line for alignment — it should flag this.
xxxxxxxxxxz :thing
x :y
short       :sym
     ^^^^^^^ Layout/SpaceBeforeFirstArg: Put one space between the method name and the first argument.

# A shared prefix like `@`, `Token`, or `&` does not count as alignment.
assert !@loader.load(generate_input(yaml, :environment => 'dev'))[:phoenix_mode]
assert  @loader.load(generate_input(yaml, :environment => 'production'))[:phoenix_mode]
      ^^ Layout/SpaceBeforeFirstArg: Put one space between the method name and the first argument.

assert !@gateway.authorize(1000, check(account_number: CHECK_FAILURE_PLACEHOLDER, number: nil)).success?
assert  @gateway.authorize(1000, check(account_number: CHECK_SUCCESS_PLACEHOLDER, number: nil)).success?
      ^^ Layout/SpaceBeforeFirstArg: Put one space between the method name and the first argument.

assert_equal 64, @app.run(['foo']), "Expected exit status to be 64"
assert  @fake_stderr.contained?(/requires these options.*flag/), @fake_stderr.strings.inspect
      ^^ Layout/SpaceBeforeFirstArg: Put one space between the method name and the first argument.
assert !@called

assert !Token.exists?(t1.id)
assert  Token.exists?(t2.id)
      ^^ Layout/SpaceBeforeFirstArg: Put one space between the method name and the first argument.

expect(@fixture.set_block  &a).to eq(a)
                         ^^ Layout/SpaceBeforeFirstArg: Put one space between the method name and the first argument.
expect(@fixture.call_block  &b).to eq(@a)
                          ^^ Layout/SpaceBeforeFirstArg: Put one space between the method name and the first argument.

b.environment  new_resource.environment
             ^^ Layout/SpaceBeforeFirstArg: Put one space between the method name and the first argument.
