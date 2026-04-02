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
