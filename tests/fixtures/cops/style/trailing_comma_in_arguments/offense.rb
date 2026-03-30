foo(1, 2, 3,)
           ^ Style/TrailingCommaInArguments: Avoid comma after the last parameter of a method call.

bar(a, b,)
        ^ Style/TrailingCommaInArguments: Avoid comma after the last parameter of a method call.

baz("hello",)
           ^ Style/TrailingCommaInArguments: Avoid comma after the last parameter of a method call.

::GraphQL::Query.new(
  schema,
  <<~END_OF_QUERY,
                 ^ Style/TrailingCommaInArguments: Avoid comma after the last parameter of a method call.
    query getPost($postSlug: String!) {
      post(slug: $postSlug) { title }
    }
  END_OF_QUERY
)

expect(schema.to_definition).to match_sdl(
  <<~GRAPHQL,
            ^ Style/TrailingCommaInArguments: Avoid comma after the last parameter of a method call.
    type Query {
      _service: _Service!
    }
  GRAPHQL
)

foo(
  body: <<~BODY,
               ^ Style/TrailingCommaInArguments: Avoid comma after the last parameter of a method call.
    hello
  BODY
)

foo(
  a: { text: <<-END },
                     ^ Style/TrailingCommaInArguments: Avoid comma after the last parameter of a method call.
content
  END
)
