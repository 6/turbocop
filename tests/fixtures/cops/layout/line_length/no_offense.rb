# frozen_string_literal: true

x = 1
y = 2
puts "hello world"
a_long_variable_name = some_method_call(arg1, arg2, arg3)

# AllowURI: a URI that extends to end of line should be allowed even if line > 120
# (the default AllowURI: true makes this OK)
some_long_variable = "see https://example.com/very/long/path/that/pushes/the/line/over/the/limit/but/extends/to/end"

# AllowQualifiedName: a qualified name (Foo::Bar::Baz) that extends to end of line should be allowed
text_document: LanguageServer::Protocol::Interface::OptionalVersionedTextDocumentIdentifier.new(

# AllowHeredoc: long lines inside a single heredoc should be allowed
msg = <<~TEXT
  This is a very long line inside a heredoc that exceeds the default maximum line length of one hundred and twenty characters easily
TEXT

# AllowHeredoc: multiple heredocs opened on the same line — content of BOTH should be allowed
expect(<<~HTML.chomp.process.first).to eq(<<~TEXT.chomp)
  <p>This is a very long HTML line inside the first heredoc that exceeds the default maximum line length of one hundred and twenty characters easily</p>
HTML
  This is a very long text line inside the second heredoc that exceeds the default maximum line length of one hundred and twenty characters without issue
TEXT

# AllowURI: URL with embedded URL in query params — the first URL starts before max and extends to end of line
      "oembed_get_request" => "http://www.flickr.com/services/oembed/?format=json&frame=1&iframe=1&maxheight=420&maxwidth=420&url=http://www.flickr.com/photos/bees/2341623661",

# Legacy block disable syntax with `Layout:LineLength` should still suppress the enclosed long line
# rubocop:disable Layout:LineLength
x = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
# rubocop:enable Layout:LineLength
