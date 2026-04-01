if(x)
^^ Layout/SpaceAroundKeyword: Space after keyword `if` is missing.
  y
end
unless(x)
^^^^^^ Layout/SpaceAroundKeyword: Space after keyword `unless` is missing.
  y
end
while(x)
^^^^^ Layout/SpaceAroundKeyword: Space after keyword `while` is missing.
  y
end
x = IO.read(__FILE__)rescue nil
                     ^^^^^^ Layout/SpaceAroundKeyword: Space before keyword `rescue` is missing.
x = 1and 2
     ^^^ Layout/SpaceAroundKeyword: Space before keyword `and` is missing.
x = 1or 2
     ^^ Layout/SpaceAroundKeyword: Space before keyword `or` is missing.
x = a.to_s; y = b.to_s; z = c if(true)
                              ^^ Layout/SpaceAroundKeyword: Space after keyword `if` is missing.
case(ENV.fetch("DIST"))
^^^^ Layout/SpaceAroundKeyword: Space after keyword `case` is missing.
when "redhat"
  puts "ok"
end
if true
  1
elsif(options.fetch(:cacheable))
^^^^^ Layout/SpaceAroundKeyword: Space after keyword `elsif` is missing.
  nil
end
x = defined?SafeYAML
    ^^^^^^^^ Layout/SpaceAroundKeyword: Space after keyword `defined?` is missing.
x = super!=true
    ^^^^^ Layout/SpaceAroundKeyword: Space after keyword `super` is missing.
f = "x"
f.chop!until f[-1] != "/"
       ^^^^^ Layout/SpaceAroundKeyword: Space before keyword `until` is missing.
def bar; return(1); end
         ^^^^^^ Layout/SpaceAroundKeyword: Space after keyword `return` is missing.
[1].each { |x|->do end.call }
                ^^ Layout/SpaceAroundKeyword: Space before keyword `do` is missing.
x = a==[]?self[m.to_s]:super
                       ^^^^^ Layout/SpaceAroundKeyword: Space before keyword `super` is missing.
# Comment ending with period.
case(ENV.fetch("DIST"))
^^^^ Layout/SpaceAroundKeyword: Space after keyword `case` is missing.
when "redhat"
  puts "ok"
end
# Sure modified files get preserved on uninstall.
if(os[:family] == "redhat")
^^ Layout/SpaceAroundKeyword: Space after keyword `if` is missing.
  puts "ok"
end
# Return them...
if(list = items.select(&:valid?)).any?
^^ Layout/SpaceAroundKeyword: Space after keyword `if` is missing.
  list.first
end
message = <<~EOS
  Actual response code: #{response.code if(response)}
                                        ^^ Layout/SpaceAroundKeyword: Space after keyword `if` is missing.
EOS

it "can make a new query with a new limit" do:w
                                           ^^ Layout/SpaceAroundKeyword: Space after keyword `do` is missing.
  nil
end

case conf[:mode]
when:new_ring
^^^^ Layout/SpaceAroundKeyword: Space after keyword `when` is missing.
  nil
end

(m..n).inject(0) do|sum, j|
                 ^^ Layout/SpaceAroundKeyword: Space after keyword `do` is missing.
  sum + j
end

before(:each)do
             ^^ Layout/SpaceAroundKeyword: Space before keyword `do` is missing.
  setup
end

RSpec.describe(SomeObject)do
                          ^^ Layout/SpaceAroundKeyword: Space before keyword `do` is missing.
  nil
end

Squib::Deck.new(width:'2in', height: '1in')do
                                           ^^ Layout/SpaceAroundKeyword: Space before keyword `do` is missing.
  nil
end

h = c.inject({})do |old, new|
                ^^ Layout/SpaceAroundKeyword: Space before keyword `do` is missing.
  old.merge!(new)
end

assert_raised_with_message("msg", RuntimeError)do
                                               ^^ Layout/SpaceAroundKeyword: Space before keyword `do` is missing.
  run
end

After('~@cli')do |scenario|
              ^^ Layout/SpaceAroundKeyword: Space before keyword `do` is missing.
  scenario
end

output = CSV.generate(:col_sep => "\t", :row_sep => "\r\n")do |csv|
                                                           ^^ Layout/SpaceAroundKeyword: Space before keyword `do` is missing.
  csv << ["x"]
end

source2evt.inject(0)do |memo, evts|
                    ^^ Layout/SpaceAroundKeyword: Space before keyword `do` is missing.
  memo + evts[1].inject(0) { |sum, h| sum + h[1].size }
end
