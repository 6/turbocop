[1, 2].each{ |x| puts x }
           ^ Layout/SpaceBeforeBlockBraces: Space missing to the left of {.
[1, 2].map{|x| x * 2 }
          ^ Layout/SpaceBeforeBlockBraces: Space missing to the left of {.
foo.select{|x| x > 1 }
          ^ Layout/SpaceBeforeBlockBraces: Space missing to the left of {.
->{puts "hello"}
  ^ Layout/SpaceBeforeBlockBraces: Space missing to the left of {.
->(){1 + 2}
    ^ Layout/SpaceBeforeBlockBraces: Space missing to the left of {.
->(x){x * 2}
     ^ Layout/SpaceBeforeBlockBraces: Space missing to the left of {.
expect(->{raise "boom"}).to raise_error
         ^ Layout/SpaceBeforeBlockBraces: Space missing to the left of {.
