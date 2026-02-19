foo.bar.baz

foo
  .bar

foo
  .bar
  .baz

obj.method1
   .method2
   .method3

# In method body
def foo
  query
    .select('foo')
    .limit(1)
end

# Block-based expect chain
expect(response)
  .to have_http_status(200)
  .and have_http_link_header('http://example.com')

# First continuation dot when all previous dots are inline
ActiveRecord::Base.configurations.configs_for(env_name: Rails.env).first.configuration_hash
  .dup
  .tap { |config| config['pool'] = 1 }

# Block chain continuation: aligned with the block-bearing call's dot
frequencies.map.with_index { |f, i| [f / total, hex[i]] }
               .sort_by { |r| -r[0] }
               .reject { |r| r[1].size == 8 }

# Hash pair value chain: correctly aligned
foo(bar: baz
         .qux
         .quux)

# Chain inside parenthesized args without hash value (RuboCop skips these)
foo(baz
      .qux
        .quux)

# Hash pair value: chain after single-line block aligns with block-call dot
foo(bar: items.reject { |e| e.nil? }
              .sort_by(&:name)
              .map(&:id))

