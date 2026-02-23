foo(
  1
)

bar(1, 2, 3)

baz(
  "hello",
  "world"
)

# Inner call in parenthesized outer call — special_for_inner_method_call_in_parentheses
Conversation.create!(conversation_params.merge(
                       contact_inbox_id: id
                     ))

expect(helper.generate_category_link(
         portal_slug: 'portal_slug',
         category_locale: 'en'
       )).to eq('/hc/portal_slug/en')

stub_const('ENV', ENV.to_hash.merge(
                    'FRONTEND_URL' => 'http://localhost:3000',
                    'NOTION_CLIENT_ID' => 'test_client_id'
                  ))

expect(cli.run(
         [
           '--autocorrect-all',
           '--only', 'Style/SoleNestedConditional'
         ]
       )).to eq(0)

expect(described_class.new(inbox: inbox).available_agent(
         allowed_agent_ids: [
           inbox_members[3].user_id
         ].map(&:to_s)
       )).to eq inbox_members[2].user
