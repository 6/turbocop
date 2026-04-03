{a: 1}
^ Layout/SpaceInsideHashLiteralBraces: Space inside { missing.
     ^ Layout/SpaceInsideHashLiteralBraces: Space inside } missing.
{b: 2, c: 3}
^ Layout/SpaceInsideHashLiteralBraces: Space inside { missing.
           ^ Layout/SpaceInsideHashLiteralBraces: Space inside } missing.
{foo: :bar}
^ Layout/SpaceInsideHashLiteralBraces: Space inside { missing.
          ^ Layout/SpaceInsideHashLiteralBraces: Space inside } missing.
{a: 1,
^ Layout/SpaceInsideHashLiteralBraces: Space inside { missing.
 b: 2}
     ^ Layout/SpaceInsideHashLiteralBraces: Space inside } missing.
x = {name: "test",
    ^ Layout/SpaceInsideHashLiteralBraces: Space inside { missing.
     role: "admin"}
                  ^ Layout/SpaceInsideHashLiteralBraces: Space inside } missing.

settings = {
  reportdir: {
    desc: "first line
    second line"},
                ^ Layout/SpaceInsideHashLiteralBraces: Space inside } missing.
}

assay_params = { description: 'first line
second line'}
            ^ Layout/SpaceInsideHashLiteralBraces: Space inside } missing.

result = { write: %{
name: Module
}}
 ^ Layout/SpaceInsideHashLiteralBraces: Space inside } missing.

add_validation({
  prompt: %{
broken
}})
 ^ Layout/SpaceInsideHashLiteralBraces: Space inside } missing.
