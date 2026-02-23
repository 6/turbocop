{ key: "value" }

{ "string_key" => "value" }

{ 1 => "one" }

{ foo: 1, bar: 2 }

x = { name: "Alice", age: 30 }

foo(option: true)

# Mixed key types — don't flag symbol keys with =>
{ "string_key" => "value", :symbol_key => 1 }

{ "@type" => "Person", :name => "Alice", :age => 30 }

# Setter method symbol key — can't use 1.9 syntax
{ :timeouts= => nil }
