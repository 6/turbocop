x = 1 ; y = 2
     ^ Layout/SpaceBeforeSemicolon: Space found before semicolon.
a = 1 ; b = 2
     ^ Layout/SpaceBeforeSemicolon: Space found before semicolon.
foo ; bar
   ^ Layout/SpaceBeforeSemicolon: Space found before semicolon.

case key
when ?\  ; toggle_view(:listing)
        ^ Layout/SpaceBeforeSemicolon: Space found before semicolon.
end
