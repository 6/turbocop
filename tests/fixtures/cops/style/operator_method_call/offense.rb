foo.+ bar
   ^ Style/OperatorMethodCall: Redundant dot detected.

foo.- 42
   ^ Style/OperatorMethodCall: Redundant dot detected.

foo.== bar
   ^ Style/OperatorMethodCall: Redundant dot detected.

dave = (0...60).map { 65.+(rand(25)).chr }.join
                        ^ Style/OperatorMethodCall: Redundant dot detected.

other_heading.instance_of?(self.class) && self.==(other_heading)
                                              ^ Style/OperatorMethodCall: Redundant dot detected.

array.-(other).length
     ^ Style/OperatorMethodCall: Redundant dot detected.

@regexp.=~(@string)
       ^ Style/OperatorMethodCall: Redundant dot detected.

# Parenthesized operator call with a bare no-receiver RHS stays offensive,
# even when nested under another call
expect(one.==(two)).to eq(true)
          ^ Style/OperatorMethodCall: Redundant dot detected.

assert(a.+(b))
        ^ Style/OperatorMethodCall: Redundant dot detected.

foo(x.-(y))
     ^ Style/OperatorMethodCall: Redundant dot detected.

bar(x.*(y), z)
     ^ Style/OperatorMethodCall: Redundant dot detected.

assert_equal 0, a.<=>(b)
                 ^ Style/OperatorMethodCall: Redundant dot detected.

assert_nil @c1.<=>(other)
              ^ Style/OperatorMethodCall: Redundant dot detected.

it { is_expected.to be_a(Class).and be.<(described_class) }
                                      ^ Style/OperatorMethodCall: Redundant dot detected.
