x = 10e6
    ^^^^ Style/ExponentialNotation: Use a mantissa >= 1 and < 10.
y = 12.34e3
    ^^^^^^^ Style/ExponentialNotation: Use a mantissa >= 1 and < 10.
z = 0.314e1
    ^^^^^^^ Style/ExponentialNotation: Use a mantissa >= 1 and < 10.

+2.5e20.round(-20).should   eql( +3 * 10 ** 20  )
^ Style/ExponentialNotation: Use a mantissa >= 1 and < 10.

+2.4e20.round(-20).should   eql( +2 * 10 ** 20  )
^ Style/ExponentialNotation: Use a mantissa >= 1 and < 10.

+2.5e200.round(-200).should eql( +3 * 10 ** 200 )
^ Style/ExponentialNotation: Use a mantissa >= 1 and < 10.

+2.4e200.round(-200).should eql( +2 * 10 ** 200 )
^ Style/ExponentialNotation: Use a mantissa >= 1 and < 10.
