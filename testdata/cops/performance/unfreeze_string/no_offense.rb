+''.dup
String.new('hello')
Array.new
+''
'str'.dup
::String.new('hello')
# Qualified constant path — different class, not flagged
ActiveModel::Type::String.new
Something::String.new
Foo::Bar::String.new('')
