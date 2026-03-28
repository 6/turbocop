foo(1 , 2)
     ^ Layout/SpaceBeforeComma: Space found before comma.
x = [1 , 2 , 3]
      ^ Layout/SpaceBeforeComma: Space found before comma.
          ^ Layout/SpaceBeforeComma: Space found before comma.
bar(a , b)
     ^ Layout/SpaceBeforeComma: Space found before comma.
yield  1 , 2
        ^ Layout/SpaceBeforeComma: Space found before comma.
next  1 ,
       ^ Layout/SpaceBeforeComma: Space found before comma.
  2
break  1  , 2
        ^^ Layout/SpaceBeforeComma: Space found before comma.

x = <<-STR
  #{response.body[[0, n - 200].max , 400]}
                                  ^ Layout/SpaceBeforeComma: Space found before comma.
STR
buffer.insert(iter, "foo\
bar" ,
    ^ Layout/SpaceBeforeComma: Space found before comma.
              :tags => ["rtl_quote"])
