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
