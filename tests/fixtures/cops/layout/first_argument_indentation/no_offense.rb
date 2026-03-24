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

# Lambda/proc end.() call — should not flag
search = lambda do |params|
  query = { match_all: {} }
  filter = nil
  if params[:q]
    query = params[:q]
  end
  if params[:t]
    filter = params[:t]
  end
  { bool: { must: [query], filter: filter } }
end.(params[:q], params[:t]),

# String interpolation with method call inside heredoc — correctly indented
content = <<~HTML
  #{builder.attachment(
    :image,
    titled: true
  )}
HTML

# Non-parenthesized call with backslash continuation — correctly indented
tag.button \
  class: "btn",
  data: { action: "messages#returnToLatest" },
  hidden: true

# Backslash continuation with correct indent
f.write \
  "some string"

# super() with correct indentation
super(
  serializer: Serializer,
  host: host,
  port: port.to_i
)

# Inner call in super() — super is not an eligible parent for
# special_for_inner_method_call_in_parentheses
def check_box(attribute, options = {})
  super(attribute, options.merge(
    label: label,
    label_options: { class: "checkbox-label" }
  ))
end

def initialize(code, body, method, url)
  super(format(
    'Response code %s for %s %s: %s',
    code, method, url, body
  ))
end

def show_topic(site_key, topic_key, options = {})
  super(site_key, topic_key, options.merge(
    pre_js: "var Config = {};"
  ))
end

label_with_hint(attribute, options) +
  super(attribute, options.merge(
    label: false, hint: nil,
    aria: { describedby: help_text_id(attribute, options) }
  ))

# Tab-indented code with correct indentation (2 tab prev + 2 = 4 tab arg)
		method_call(
				arg1,
				arg2
		)
