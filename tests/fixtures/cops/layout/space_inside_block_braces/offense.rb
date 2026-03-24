[1, 2].each {|x| puts x}
            ^^ Layout/SpaceInsideBlockBraces: Space between { and | missing.
                       ^ Layout/SpaceInsideBlockBraces: Space missing inside }.
[1, 2].map {|x| x * 2}
           ^^ Layout/SpaceInsideBlockBraces: Space between { and | missing.
                     ^ Layout/SpaceInsideBlockBraces: Space missing inside }.
foo.select {|x| x > 1}
           ^^ Layout/SpaceInsideBlockBraces: Space between { and | missing.
                     ^ Layout/SpaceInsideBlockBraces: Space missing inside }.
escape_html = ->(str) {str.gsub("&", "&amp;")}
                      ^ Layout/SpaceInsideBlockBraces: Space missing inside {.
                                             ^ Layout/SpaceInsideBlockBraces: Space missing inside }.
has_many :versions, -> {order("id ASC")}, class_name: "Foo"
                       ^ Layout/SpaceInsideBlockBraces: Space missing inside {.
                                       ^ Layout/SpaceInsideBlockBraces: Space missing inside }.
action = -> {puts "hello"}
            ^ Layout/SpaceInsideBlockBraces: Space missing inside {.
                         ^ Layout/SpaceInsideBlockBraces: Space missing inside }.
