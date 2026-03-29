"#{x}"
^^^^^^ Style/RedundantInterpolation: Prefer `to_s` over string interpolation.

"#{obj.method}"
^^^^^^^^^^^^^^^ Style/RedundantInterpolation: Prefer `to_s` over string interpolation.

"#{foo}"
^^^^^^^^ Style/RedundantInterpolation: Prefer `to_s` over string interpolation.

unused_port "#@test6_addr"
            ^^^^^^^^^^^^^^ Style/RedundantInterpolation: Prefer `to_s` over string interpolation.

assert_equal("hello".b << ("ffff" * 4096 * 3) << "#$/", line)
                                                 ^^^^^ Style/RedundantInterpolation: Prefer `to_s` over string interpolation.

assert_equal("hello".b << ("ffff" * 4096 * 3) << "#$/", line)
                                                 ^^^^^ Style/RedundantInterpolation: Prefer `to_s` over string interpolation.

@buffer << "#{"[ #{title} ]"}"
           ^^^^^^^^^^^^^^^^^^^ Style/RedundantInterpolation: Prefer `to_s` over string interpolation.

$1 ? UNESCAPE[$1] : [ "#$2".hex ].pack('U*')
                      ^^^^^ Style/RedundantInterpolation: Prefer `to_s` over string interpolation.

"#@prefix"
^ Style/RedundantInterpolation: Prefer `to_s` over string interpolation.

find("#@dir").must_equal target
     ^^^^^^^ Style/RedundantInterpolation: Prefer `to_s` over string interpolation.

"#@@a"
^ Style/RedundantInterpolation: Prefer `to_s` over string interpolation.
