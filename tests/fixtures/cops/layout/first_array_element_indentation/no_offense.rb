x = [
  1,
  2,
  3
]

y = [1, 2, 3]

z = []

# special_inside_parentheses: array arg with [ on same line as (
foo([
      :bar,
      :baz
    ])

method_call(arg1, [
              :first,
              :second
            ])

expect(cli.run([
                 '--autocorrect-all',
                 '--only', 'Style/HashSyntax'
               ])).to eq(0)

create(:record, value: [
         { source_id: '1', inbox: inbox },
         { source_id: '2', inbox: inbox2 }
       ])

deeply.nested.call([
                     :a,
                     :b
                   ])

# Array with method chain uses line-relative indent
expect(x).to eq([
  'hello',
  'world'
].join("\n"))

# Array in grouping paren with operator uses line-relative indent
X = (%i[
  a
  b
] + other).freeze

# Array as RHS of % operator inside method call
gc.draw('text %d,%d %s' % [
  left.round + 2,
  header_height + 14,
  shell_quote(week_f.to_s)
])

# Indented % operator array in method body
  image.draw('rectangle %d,%d %d,%d' % [
    0, 0, width, height
  ])

# Array inside hash arg that is chained with .to_json (line-relative)
  client.should_receive(:api_post).
    with(endpoint, { requests: [
      { method: 'POST', url: 'v1.0/objects/Foo' }
    ], flag: true }.to_json).
    and_return(response)

# Another chained hash pattern
foo(status: 200, body: { responses: [
  { code: 200 },
  { code: 201 }
], total: 2 }.to_json)

# Array inside hash value with .compact chain (array is chained, not direct arg)
assert_equal({ "c" => [
  { "v" => 1421218800000, "f" => "Wed, Jan 14, 2015" },
  { "v" => 2, "f" => "2" },
].compact }, data["hits_over_time"]["rows"][1])

# Array in grouping parens with + operator and .shelljoin
command = (PREFIX + %W[
  convert
  #{image}
  -coalesce
]).shelljoin

# Array in grouping parens with + operator and .freeze
VALID_CONNECTION_KEYS = (VALID_REQUEST_KEYS + %i[
  ciphers
  client_key
  client_cert
]).freeze

# Array in grouping parens with - operator and .map
all_instances = (all_types - [
  PTypeReferenceType,
  PTypeAliasType
]).map { |c| c::DEFAULT }

# Array as hash value in multi-pair hash (no parens) - hash key relative indent
foo 1, bar: [
         2,
       ],
       baz: 3

# Array as hash value in multi-pair hash (nested)
[
  { subscription_line_items_attributes: [
      :id, :quantity, :variant_id, :price_estimate, :_destroy
    ],
    bill_address_attributes: Address.attributes,
    ship_address_attributes: Address.attributes }
]

# Array as hash value in multi-pair hash assignment
FILES = { ruby: [
            "app/**/*.rb",
            "lib/**/*.rake",
          ],
          js: [
            "app/assets/**/*.js",
          ] }

# Array as keyword arg value in method call (no parens)
acts_as_searchable columns: [
                     "#{table_name}.title",
                     "#{table_name}.notes"
                   ],
                   include: [:project],
                   date_column: "#{table_name}.created_at"

# Array with inner array chained with .join inside string interpolation
regex = [
  "[\"]([^\"]+)\"",
  "%(?:#{[
    '([\\W_])([^\\4]*)\\4',
    '\[([^\\]]*)\]',
  ].join('|')})"
].join('|')

# FP fix: String argument containing - should not prevent paren-relative indent
check_order(".section__in-favor", [
              highest_voted,
              lowest_voted
            ])

# FP fix: Lambda -> should not be treated as binary operator
reduce_until(->(ctx) { ctx.number == 3 }, [
               AddOneAction,
               AddTwoAction
             ])

# FP fix: String containing / should not prevent paren-relative indent
site.pages << JsonPage.of_raw_data(site, '/', [
                                     { name: "products" },
                                     { name: "categories" }
                                   ])

# FP fix: Splat *[ inside method call parens (paren-relative)
List.new(:BULLET, *[
           ListItem.new(nil, Paragraph.new('l1')),
           ListItem.new(nil, Paragraph.new('l2'))
         ])

# FP fix: Grouping paren with space before ( — hash value array (line-relative)
assert_equal ({ "attributes" => [
  { "key" => "content", "value" => "old" },
  { "key" => "title",   "value" => "old" }
] }), record.data

# FP fix: Ternary ? between ( and [ — grouping paren (line-relative)
result = (flag ? [
  { name: item, path: resolve(item) }.compact
] : nil)

# FP fix: Grouping paren ([ — no method name before paren (line-relative)
    handler { ([
      { token: 'user', email: 'user@test.com' },
      { token: 'admin', email: 'admin@test.com' }
    ]) }

# FP fix: First element on same line as [ — skip closing bracket check
tests = [ 'tests/resource/file/content_attribute.rb',
          'tests/language/functions_in_puppet_language.rb',
          'tests/resource/service/puppet_service_management.rb'
        ]

# FP fix: First element on same line as [ with .each chain
["if /* comment */\nif",
  "if /* comment\n */\nif",
  "if /*\n comment\n */\nif",
  ].each do |source|
  something(source)
end

# FP fix: First element on same line as [ with .join chain
expect(result).to eq([ 'path: [0] value: 1',
          'path: [1] value: 2',
          ''
          ].join("\n"))

# FP fix: %w{ with first element on same line — closing } check skipped
equivalent = %w{ http://example.com/
                    http://exa%6Dple.com/
                    http://example.com:80/
                  }

# FP fix: Hash value array with first element on same line as [
[{ "organization" => { "name" => "non-admin-member" } },
 { "organization" => { "name" => "solo-admin-member" } },
]

# FP fix: First element on same line with .freeze
WIN_AUDIT_SUBCATEGORIES = ["Account Lockout",
                                 "Sensitive Privilege Use",
                                 "User Account Management",
                                ].freeze

# FP fix: Single-pair hash value with paren-relative — no intermediate method call
# paren_col=17, indent_base=18, element_col=20(=18+2), close_col=18
FactoryBot.create(:limited_admin, :groups => [
                    FactoryBot.create(:google_admin_group),
                  ])

# FP fix: Ternary ? in preceding argument, not grouping operator
@product = create(:product, recommendable ? :recommendable : nil, recurrence_price_values: [
                    {
                      BasePrice::Recurrence::MONTHLY => { enabled: true, price: 3 },
                    },
                  ])

# FP fix: Single-pair hash value with intermediate method call (`.`)
# expect(client.search body: [...]) — paren is from expect(), not search
# RuboCop uses line-relative (on_array path), so closing bracket at line indent is OK
expect(client.search body: [
  { index: 'foo', query: { match_all: {} } },
  { index: 'bar', query: { match: { foo: 'bar' } } }
])

# FP fix: explicit `super(...)` uses line-relative indentation, not paren-relative
def as_json(options = {})
  h = super(:only => [
    :short_id,
    :created_at,
    :updated_at,
    :is_deleted,
    :is_moderated,
  ])
end

# FP fix: bracketless nested call inside outer parens stays line-relative
      Motion::Project::App.any_instance.expects(:files=).with(expand_paths [
        MotionBundler::MOTION_BUNDLER_FILE,
        motion_bundler_file("motion-bundler/simulator/boot.rb"),
        "/Users/paulengel/bar.rb"
      ])

# FP fix: complex `=>` hash keys use pair-relative indentation, not the key expression tail
spec_repos = {
  Source.new(fixture('spec-repos/trunk')) => [
    Specification.new,
  ],
  OtherSource.new(fixture('spec-repos/trunk')) => [
    Specification.new,
  ],
}

# FP fix: right sibling on same line keeps array line-relative inside a hash value
bad_query = { prop: 'imageinfo', iiprop: 'url', iiurlheight: 480, pageids: [
  107709976, 111662244, 109767821,
  109782180, 109782183
], iilimit: 50 }

# Empty arrays
a = []

# Empty array with closing bracket on next line at correct indent
a << [
]
