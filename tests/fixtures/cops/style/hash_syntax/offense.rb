{ :key => "value" }
  ^^^^ Style/HashSyntax: Use the new Ruby 1.9 hash syntax.

{ :foo => 1, :bar => 2 }
  ^^^^ Style/HashSyntax: Use the new Ruby 1.9 hash syntax.
             ^^^^ Style/HashSyntax: Use the new Ruby 1.9 hash syntax.

x = { :name => "Alice", :age => 30 }
      ^^^^^ Style/HashSyntax: Use the new Ruby 1.9 hash syntax.
                        ^^^^ Style/HashSyntax: Use the new Ruby 1.9 hash syntax.

foo(:option => true)
    ^^^^^^^ Style/HashSyntax: Use the new Ruby 1.9 hash syntax.
