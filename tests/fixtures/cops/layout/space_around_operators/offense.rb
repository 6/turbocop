x =1
  ^ Layout/SpaceAroundOperators: Surrounding space missing for operator `=`.
x ==""
  ^^ Layout/SpaceAroundOperators: Surrounding space missing for operator `==`.
x= 1
 ^ Layout/SpaceAroundOperators: Surrounding space missing for operator `=`.
x!= y
 ^^ Layout/SpaceAroundOperators: Surrounding space missing for operator `!=`.
a =>"hello"
  ^^ Layout/SpaceAroundOperators: Surrounding space missing for operator `=>`.
x +y
  ^ Layout/SpaceAroundOperators: Surrounding space missing for operator `+`.
x- y
 ^ Layout/SpaceAroundOperators: Surrounding space missing for operator `-`.
x *y
  ^ Layout/SpaceAroundOperators: Surrounding space missing for operator `*`.
x &&y
  ^^ Layout/SpaceAroundOperators: Surrounding space missing for operator `&&`.
x ||y
  ^^ Layout/SpaceAroundOperators: Surrounding space missing for operator `||`.
x  && y
   ^^ Layout/SpaceAroundOperators: Operator `&&` should be surrounded by a single space.

# Compound assignment operators
x +=0
  ^^ Layout/SpaceAroundOperators: Surrounding space missing for operator `+=`.
y -=0
  ^^ Layout/SpaceAroundOperators: Surrounding space missing for operator `-=`.
z *=2
  ^^ Layout/SpaceAroundOperators: Surrounding space missing for operator `*=`.
x ||=0
  ^^^ Layout/SpaceAroundOperators: Surrounding space missing for operator `||=`.
y &&=0
  ^^^ Layout/SpaceAroundOperators: Surrounding space missing for operator `&&=`.

# Match operators
x =~/abc/
  ^^ Layout/SpaceAroundOperators: Surrounding space missing for operator `=~`.
y !~/abc/
  ^^ Layout/SpaceAroundOperators: Surrounding space missing for operator `!~`.

# Class inheritance
class Foo<Bar
         ^ Layout/SpaceAroundOperators: Surrounding space missing for operator `<`.
end

# Singleton class
class<<self
     ^^ Layout/SpaceAroundOperators: Surrounding space missing for operator `<<`.
end

# Rescue =>
begin
rescue Exception=>e
                ^^ Layout/SpaceAroundOperators: Surrounding space missing for operator `=>`.
end

# Triple equals
Hash===z
    ^^^ Layout/SpaceAroundOperators: Surrounding space missing for operator `===`.

# Exponent with spaces (default no_space style should flag)
x = a * b ** 2
          ^^ Layout/SpaceAroundOperators: Space around operator `**` detected.

# Setter call without spaces
x.y =2
    ^ Layout/SpaceAroundOperators: Surrounding space missing for operator `=`.

# Extra spaces around = (not aligned)
x  = 1
   ^ Layout/SpaceAroundOperators: Operator `=` should be surrounded by a single space.

# Extra spaces around => (not aligned)
{'key'  => 'val'}
        ^^ Layout/SpaceAroundOperators: Operator `=>` should be surrounded by a single space.

'arrow'               => [:arrow, :down],
                      ^^ Layout/SpaceAroundOperators: Operator `=>` should be surrounded by a single space.

@_apipie_dsl_data =  {
                  ^ Layout/SpaceAroundOperators: Operator `=` should be surrounded by a single space.

result =  [{
       ^ Layout/SpaceAroundOperators: Operator `=` should be surrounded by a single space.

html_block =    if render_partial?
           ^ Layout/SpaceAroundOperators: Operator `=` should be surrounded by a single space.

message[:bcc] =           'mikel@bcc.lindsaar.net'
              ^ Layout/SpaceAroundOperators: Operator `=` should be surrounded by a single space.
message[:cc] =            'mikel@cc.lindsaar.net'
             ^ Layout/SpaceAroundOperators: Operator `=` should be surrounded by a single space.
