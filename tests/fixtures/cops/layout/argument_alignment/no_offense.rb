foo(1,
    2,
    3)
bar(:a,
    :b,
    :c)
baz("x",
    "y")
single_arg(1)

# Argument after closing brace of multiline hash (not first on its line)
enum :action, {
  none: 0,
  disable: 1_000,
}, suffix: :action

# Multiple arguments on one line after a multiline arg
contain_exactly(a_hash_including({
  name: 'bar',
}), a_hash_including({
  name: 'foo',
}))

# Bracket assignment []= is skipped by RuboCop
options['pre_chat_fields'][index] =
  field.deep_merge({
                     'label' => attribute['display_name'],
                     'placeholder' => attribute['display_name']
                   })

# Keyword args after **splat on same line — aligned with each other, not the splat
described_class.new(**default_attrs, index: 1,
                                     name: 'stash',
                                     branch: 'feature',
                                     message: 'WIP on feature')

# **splat followed by keyword args on continuation lines
redirect_to checkout_url(**params, host: DOMAIN, product: permalink,
                                   rent: item[:rental], recurrence: item[:recurrence],
                                   price: item[:price],
                                   code: code,
                                   affiliate_id: params[:id])

# **splat with two-space indented continuation
deprecate(**[:store, :update].index_with(MESSAGE),
  deprecator: ActiveResource.deprecator)

# Block arg &block aligned with first argument
tag.public_send tag_element,
                class: token_list(name, classes),
                data: { controller: "pagination" },
                **properties,
                &block

# Multi-arg call with keyword hash continuation — not expanded in with_first_argument.
# The kwHash starts on the same line as the first arg, so its continuation lines
# are not individually checked.
gem "select2-rails", github: "org/select2-rails",
                     branch: "v349"

# Multiple kwargs on same line as first positional arg
redirect_to root_path, notice: "Success",
                       status: :moved_permanently

# Multi-arg with trailing keyword hash starting on same line
create :user, :admin, name: "Admin",
                      role: "superuser"

# Sole keyword hash with block pass on continuation remains allowed
render(layout: "shared/section_table",
       locals: {title: title, collection: collection, add_path: add_path},
  &block)

# Sole keyword hash arg with block pass on continuation — RuboCop expands to pairs only,
# block pass is not checked for alignment (only 1 pair, nothing to compare)
h1 = @model.document.add_listener(:before => :new_mirror,
      &method(:update_grammar))

# Wide Unicode characters before interpolation affect display width, but the
# continued argument line is still aligned under RuboCop's display column rules.
msg = "🌊 #{distance_of_time_in_words(Time.current - seconds.seconds,
                                      Time.current)} in this session."

# Same root cause with a different method call inside interpolation.
msg = "🌌 #{pluralize(hours.round(1),
                      'hour')} of suspended existence."
