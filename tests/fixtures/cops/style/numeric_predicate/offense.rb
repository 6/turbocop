foo == 0
^^^^^^^^ Style/NumericPredicate: Use `foo.zero?` instead of `foo == 0`.

bar.baz > 0
^^^^^^^^^^^ Style/NumericPredicate: Use `bar.baz.positive?` instead of `bar.baz > 0`.

0 > foo
^^^^^^^ Style/NumericPredicate: Use `foo.negative?` instead of `0 > foo`.

color == 0x00
^^^^^^^^^^^^^ Style/NumericPredicate: Use `color.zero?` instead of `color == 0x00`.

cmd >> 4 == 0b0000
^^^^^^^^^^^^^^^^^^ Style/NumericPredicate: Use `(cmd >> 4).zero?` instead of `cmd >> 4 == 0b0000`.

0x0 < byte
^^^^^^^^^^ Style/NumericPredicate: Use `byte.positive?` instead of `0x0 < byte`.

r_val[0] == 0x00
^^^^^^^^^^^^^^^^ Style/NumericPredicate: Use `(r_val[0]).zero?` instead of `r_val[0] == 0x00`.

a(value) == 0x00000000
^^^^^^^^^^^^^^^^^^^^^^ Style/NumericPredicate: Use `a(value).zero?` instead of `a(value) == 0x00000000`.

t.getbyte(3).should == 0x00
^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/NumericPredicate: Use `t.getbyte(3).should.zero?` instead of `t.getbyte(3).should == 0x00`.

image.cast("int").conv([[1, -1]]).crop(1, 0, hash_size, hash_size).>(0)./(255).cast("uchar").to_a.join.to_i(2)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/NumericPredicate: Use `image.cast("int").conv([[1, -1]]).crop(1, 0, hash_size, hash_size).positive?` instead of `image.cast("int").conv([[1, -1]]).crop(1, 0, hash_size, hash_size).>(0)`.

case [s[:commission_from_seller].>(0).or_else(false), s[:minimum_transaction_fee_cents].>(0).or_else(false)]
      ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/NumericPredicate: Use `(s[:commission_from_seller]).positive?` instead of `s[:commission_from_seller].>(0)`.
                                                      ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/NumericPredicate: Use `(s[:minimum_transaction_fee_cents]).positive?` instead of `s[:minimum_transaction_fee_cents].>(0)`.
