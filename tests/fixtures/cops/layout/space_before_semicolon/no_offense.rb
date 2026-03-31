x = 1; y = 2
a = 1; b = 2
foo; bar
s = "hello ; world"
r = /semi ; colon/

while ready
  ;
end

# Block braces with semicolon (handled by SpaceInsideBlockBraces, not SpaceBeforeSemicolon)
command("test") { ; }
app = Shoes.app { ; }
session.within_frame { ; }
let(:opts) { ; { name: "plata" } }

case key
when ?\ ; toggle_view(:listing)
end
