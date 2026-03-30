class Foo
  def bar
    <<~SQL
      'Hi'
  SQL
  ^^^ Layout/ClosingHeredocIndentation: `SQL` is not aligned with `<<~SQL`.
  end
end

class Baz
  def qux
    <<~RUBY
      something
        RUBY
        ^^^^ Layout/ClosingHeredocIndentation: `RUBY` is not aligned with `<<~RUBY`.
  end
end

def example
  <<-TEXT
    hello
      TEXT
      ^^^^ Layout/ClosingHeredocIndentation: `TEXT` is not aligned with `<<-TEXT`.
end

# Heredoc in block body should still flag offense
get '/foo' do
    <<-EOHTML
    <html></html>
EOHTML
^^^^^^ Layout/ClosingHeredocIndentation: `EOHTML` is not aligned with `<<-EOHTML`.
end

# Heredoc as argument with wrong closing alignment (matches neither opening nor call)
include_examples :offense,
                 <<-HEREDOC
  bar
    HEREDOC
    ^^^^^^^ Layout/ClosingHeredocIndentation: `HEREDOC` is not aligned with `<<-HEREDOC` or beginning of method definition.

# Heredoc hash values should not align to the outermost call
create_dynamic_portlet(:recently_updated_pages,
                       :template => <<-TEMPLATE
  body
TEMPLATE
^ Layout/ClosingHeredocIndentation: `TEMPLATE` is not aligned with `:template => <<-TEMPLATE`.
)

# Keyword heredoc values should not align to the outermost call either
Fabricate(
  :theme_field,
  value: <<~HTML,
    <script></script>
HTML
^ Layout/ClosingHeredocIndentation: `HTML` is not aligned with `value: <<~HTML,`.
)

# Trailing keyword args after the heredoc do not change the indentation rule
second_migration_field =
  Fabricate(
    :migration_theme_field,
    value: <<~JS,
      export default function migrate(settings) {
        settings.set("integer_setting", 3);
        return settings;
      }
JS
^^ Layout/ClosingHeredocIndentation: `JS` is not aligned with `value: <<~JS,`.
    version: 1,
  )
