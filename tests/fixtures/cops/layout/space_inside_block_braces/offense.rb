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

class ForwardingSuperExamples
  def each
    super {|*a| sleep 0.1 ; yield(*a) }
          ^^ Layout/SpaceInsideBlockBraces: Space between { and | missing.
  end

  def union_to_s
    super {['every ',' or ']}
          ^ Layout/SpaceInsideBlockBraces: Space missing inside {.
                            ^ Layout/SpaceInsideBlockBraces: Space missing inside }.
  end

  def intersect_to_s
    super {['every ', ' and ']}
          ^ Layout/SpaceInsideBlockBraces: Space missing inside {.
                              ^ Layout/SpaceInsideBlockBraces: Space missing inside }.
  end

  def method_4(&block)
    super {|y| block.call(y)}
          ^^ Layout/SpaceInsideBlockBraces: Space between { and | missing.
                            ^ Layout/SpaceInsideBlockBraces: Space missing inside }.
  end

  def downto(min)
    super {|dt| yield dt.clear_timezone_offset }
          ^^ Layout/SpaceInsideBlockBraces: Space between { and | missing.
  end

  def step(limit, step = 1)
    super {|dt| yield dt.clear_timezone_offset }
          ^^ Layout/SpaceInsideBlockBraces: Space between { and | missing.
  end

  def upto(max)
    super {|dt| yield dt.clear_timezone_offset }
          ^^ Layout/SpaceInsideBlockBraces: Space between { and | missing.
  end
end
