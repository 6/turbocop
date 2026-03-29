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

p(class: 'intro') {"
                  ^ Layout/SpaceInsideBlockBraces: Space missing inside {.
hello
"}

p {"
  ^ Layout/SpaceInsideBlockBraces: Space missing inside {.
hello
"}

p(class: 'conclusion') {"
                       ^ Layout/SpaceInsideBlockBraces: Space missing inside {.
hello
"}

p(class: 'legend') {"
                   ^ Layout/SpaceInsideBlockBraces: Space missing inside {.
hello
"}

audit_options {{
              ^ Layout/SpaceInsideBlockBraces: Space missing inside {.
  foo: :bar
}}

let(:domains) {[
              ^ Layout/SpaceInsideBlockBraces: Space missing inside {.
  { domain: 'example.com' }
]}

let(:source) {<<-CODE
             ^ Layout/SpaceInsideBlockBraces: Space missing inside {.
body
CODE
}

before {FlavourSaver.register_helper(:repeat) do |a, &block|
       ^ Layout/SpaceInsideBlockBraces: Space missing inside {.
  a.times { block.call }
end}
