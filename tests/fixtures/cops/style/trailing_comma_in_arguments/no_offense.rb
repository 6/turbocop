foo(1, 2, 3)

foo(1)

foo

bar(a, b)

baz("hello")

::GraphQL::Query.new(
  schema,
  <<~END_OF_QUERY
    query getPost($postSlug: String!) {
      post(slug: $postSlug) { title }
    }
  END_OF_QUERY
)

expect(schema.to_definition).to match_sdl(
  <<~GRAPHQL
    type Query {
      _service: _Service!
    }
  GRAPHQL
)

foo(
  body: <<~BODY
    hello
  BODY
)
