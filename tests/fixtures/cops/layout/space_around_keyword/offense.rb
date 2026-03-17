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
