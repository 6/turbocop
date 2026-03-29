x = {
      a: 1,
      ^^^ Layout/FirstHashElementIndentation: Use 2 (not 6) spaces for indentation of the first element.
  b: 2
}
y = {
    c: 3,
    ^^ Layout/FirstHashElementIndentation: Use 2 (not 4) spaces for indentation of the first element.
  d: 4
}
z = {
        e: 5,
        ^^^ Layout/FirstHashElementIndentation: Use 2 (not 8) spaces for indentation of the first element.
  f: 6
}

buffer << {
  }
  ^ Layout/FirstHashElementIndentation: Indent the right brace the same as the start of the line where the left brace is.

value = {
  a: 1
    }
    ^ Layout/FirstHashElementIndentation: Indent the right brace the same as the start of the line where the left brace is.

wrap({
       a: 1
    })
    ^ Layout/FirstHashElementIndentation: Indent the right brace the same as the first position after the preceding left parenthesis.

func(x: {
       a: 1,
       b: 2
   },
   ^ Layout/FirstHashElementIndentation: Indent the right brace the same as the parent hash key.
     y: {
       c: 1,
       d: 2
     })

# Hash inside double-splat (**{}) in method call — first element wrong indent
# paren at col 9, base = 9+1=10, expected = 10+2=12, actual = 4
translate('msg', **{
    :key => 'val',
    ^^ Layout/FirstHashElementIndentation: Use 2 (not 0) spaces for indentation of the first element.
    :cls => klass.to_s
          })

# Hash inside double-splat — right brace wrong indent
# paren at col 9, expected closing = 10
translate('msg', **{
                   :key => 'val',
                   ^^^ Layout/FirstHashElementIndentation: Use 2 (not 9) spaces for indentation of the first element.
                   :cls => klass.to_s
  })
  ^ Layout/FirstHashElementIndentation: Indent the right brace the same as the first position after the preceding left parenthesis.

# Hash inside local var assignment in method args
# paren at col 21, base = 21+1=22, expected = 22+2=24, actual = 4
migration.proper_name(table, options = {
    prefix: Base.prefix,
    ^^ Layout/FirstHashElementIndentation: Use 2 (not 0) spaces for indentation of the first element.
    suffix: Base.suffix
  })
  ^ Layout/FirstHashElementIndentation: Indent the right brace the same as the first position after the preceding left parenthesis.

# Hash inside ternary in method call args
# paren at col 20, expected closing = 21
Autoprefixer.install(self, safe ? config : {
  })
  ^ Layout/FirstHashElementIndentation: Indent the right brace the same as the first position after the preceding left parenthesis.

# Hash inside || expression in a parenthesized method call
ActiveRecord::Base.establish_connection(ENV['DATABASE_URL'] || {
  adapter: 'postgresql',
  ^^^^^^^ Layout/FirstHashElementIndentation: Use 2 (not 0) spaces for indentation of the first element.
  username: 'travis',
  port: 5433,
})
^ Layout/FirstHashElementIndentation: Indent the right brace the same as the first position after the preceding left parenthesis.

# Hash returned from a block body inside a parenthesized method call
expect(list.map { |item| {
  kind: item.kind,
  ^^^^ Layout/FirstHashElementIndentation: Use 2 (not 0) spaces for indentation of the first element.
  namespace: item.metadata.namespace,
  name: item.metadata.name,
} }).to match [
^ Layout/FirstHashElementIndentation: Indent the right brace the same as the first position after the preceding left parenthesis.
  { kind: "Node", namespace: nil, name: "ubuntu-xenial" }
]

# Hash inside || expression in a constructor call
plugin = Thor::CoreExt::HashWithIndifferentAccess.new(config[:host_plugin] || {
  'type' => 'file',
  ^^^^^^ Layout/FirstHashElementIndentation: Use 2 (not 0) spaces for indentation of the first element.
  'path' => 'hosts.yml'
})
^ Layout/FirstHashElementIndentation: Indent the right brace the same as the first position after the preceding left parenthesis.

# Right brace inside a do..end block argument to a parenthesized call
wrap(items.map do |item| {
       id: item.id
    }
    ^ Layout/FirstHashElementIndentation: Indent the right brace the same as the first position after the preceding left parenthesis.
end)
