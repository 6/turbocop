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

# Quoted symbol keys — can use "key": syntax (Ruby >= 2.2)
{ :"chef version" => 1, :name => 2 }
  ^^^^^^^^^^^^^^^^ Style/HashSyntax: Use the new Ruby 1.9 hash syntax.
                        ^^^^^ Style/HashSyntax: Use the new Ruby 1.9 hash syntax.

foo(:name => id, :"spaces here" => val)
    ^^^^^ Style/HashSyntax: Use the new Ruby 1.9 hash syntax.
                 ^^^^^^^^^^^^^^^ Style/HashSyntax: Use the new Ruby 1.9 hash syntax.

# Interpolated symbol keys — Prism uses InterpolatedSymbolNode, but these are
# still convertible to quoted label syntax on Ruby >= 2.2.
task :"setup:#{provider}" => File.join(ARTIFACT_DIR, "#{provider}.box")
     ^^^^^^^^^^^^^^^^^^^^ Style/HashSyntax: Use the new Ruby 1.9 hash syntax.

{ :"#{app_name}-orchestrated-by" => pod_name }
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/HashSyntax: Use the new Ruby 1.9 hash syntax.

record.update(:"has_#{record.class.table_name}_poly_type" => "PolyBadRecord")
              ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/HashSyntax: Use the new Ruby 1.9 hash syntax.

update(:"#{self.class.table_name}_belongs_to_poly_type" => "PolyBadRecord")
       ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/HashSyntax: Use the new Ruby 1.9 hash syntax.

{ :"#{field}_string" => nil }
  ^^^^^^^^^^^^^^^^^^ Style/HashSyntax: Use the new Ruby 1.9 hash syntax.

# Quoted symbol keys with 1.9-style siblings — rocket keys should still be flagged
{ "font-variant": "normal",
  :'font-style' => "italic",
  ^^^^^^^^^^^^^ Style/HashSyntax: Use the new Ruby 1.9 hash syntax.
  :'letter-spacing' => "2px",
  ^^^^^^^^^^^^^^^^^ Style/HashSyntax: Use the new Ruby 1.9 hash syntax.
  :'vertical-align' => "top" }
  ^^^^^^^^^^^^^^^^^ Style/HashSyntax: Use the new Ruby 1.9 hash syntax.
