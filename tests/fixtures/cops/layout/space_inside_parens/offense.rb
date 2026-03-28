( x + 1 )
 ^ Layout/SpaceInsideParens: Space inside parentheses detected.
       ^ Layout/SpaceInsideParens: Space inside parentheses detected.
( y )
 ^ Layout/SpaceInsideParens: Space inside parentheses detected.
   ^ Layout/SpaceInsideParens: Space inside parentheses detected.
( z)
 ^ Layout/SpaceInsideParens: Space inside parentheses detected.
foo( bar)
    ^ Layout/SpaceInsideParens: Space inside parentheses detected.
baz( x, y )
    ^ Layout/SpaceInsideParens: Space inside parentheses detected.
         ^ Layout/SpaceInsideParens: Space inside parentheses detected.

def configure( options )
              ^ Layout/SpaceInsideParens: Space inside parentheses detected.
                      ^ Layout/SpaceInsideParens: Space inside parentheses detected.
  deliver( payload,
          ^ Layout/SpaceInsideParens: Space inside parentheses detected.
           format: :json )
                        ^ Layout/SpaceInsideParens: Space inside parentheses detected.
end

def matches?(value)
  super( value )
        ^ Layout/SpaceInsideParens: Space inside parentheses detected.
              ^ Layout/SpaceInsideParens: Space inside parentheses detected.
end

if defined?( value )
            ^ Layout/SpaceInsideParens: Space inside parentheses detected.
                  ^ Layout/SpaceInsideParens: Space inside parentheses detected.
  matches?(value)
end

check ( value )
             ^ Layout/SpaceInsideParens: Space inside parentheses detected.

x = flag ? ( value) : other
            ^ Layout/SpaceInsideParens: Space inside parentheses detected.

foo { | ( x ) , z | }
         ^ Layout/SpaceInsideParens: Space inside parentheses detected.
           ^ Layout/SpaceInsideParens: Space inside parentheses detected.

case value
  in ^ ( 1 + 2 )
        ^ Layout/SpaceInsideParens: Space inside parentheses detected.
              ^ Layout/SpaceInsideParens: Space inside parentheses detected.
  1
end

yield( 1 , 2 )
      ^ Layout/SpaceInsideParens: Space inside parentheses detected.
            ^ Layout/SpaceInsideParens: Space inside parentheses detected.

case value
  in Point(  1, Integer => a)
           ^ Layout/SpaceInsideParens: Space inside parentheses detected.
    a
end

case value
  in SuperPoint(   x: 0.. => px)
                ^ Layout/SpaceInsideParens: Space inside parentheses detected.
    px
end

( x, y ) = [1, 2]
 ^ Layout/SpaceInsideParens: Space inside parentheses detected.
      ^ Layout/SpaceInsideParens: Space inside parentheses detected.

(receipt, ) = foo
         ^ Layout/SpaceInsideParens: Space inside parentheses detected.

include_node?(node) and ( child.nil? or child.equal?(Nothing) or
                         ^ Layout/SpaceInsideParens: Space inside parentheses detected.

return "#{set}call0(#{obj.name},#{meth.id});" if obj and ( params.empty? or params[0]==nil or params[0]==obj)
                                                          ^ Layout/SpaceInsideParens: Space inside parentheses detected.
