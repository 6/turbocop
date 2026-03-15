items.each do |x|
  puts x
end

items.each { |x| puts x }

[1, 2].map do |x|
  x * 2
end

# end aligned with chain expression start (not the do-line)
@source_account.passive_relationships
               .where(account: Account.local)
               .in_batches do |follows|
  follows.update_all(target_account_id: 1)
end

# end aligned with call expression start in a hash value
def generate
  {
    data: items.map do |item|
            item.to_s
          end,
  }
end

# end aligned with call on previous line via backslash continuation
it 'does something' \
   'very interesting' do
  run_test
end

# end aligned with call on previous line via multiline args
option(opts, '--fail-level SEVERITY',
       RuboCop::Cop::Severity::NAMES) do |severity|
  @options[:fail_level] = severity
end

# end aligned with call expression that has multiline args ending with comma
add_offense(node,
            message: format(MSG,
                            flag: true)) do |corrector|
  corrector.replace(node, replacement)
end

# Multiline %i[] array with .each do block — end aligns with %i[
%i[opposite_style_detected unexpected_style_detected
   ambiguous_style_detected conflicting_styles_detected
   unrecognized_style_detected
   no_acceptable_style!].each do |method|
  puts method
end

# do...end block inside a brace block — end aligns with chain root
to = lambda { |env|
  hostess.call(env)
    .tap do |response|
      response[1].delete("x-cascade")
  end
}

# string concatenation with + on multiline method description (RSpec-style)
it "should convert " +
    "some value " +
    "correctly" do
  run_test
end

# string concatenation with + on multiline describe block
describe User, "when created with a name known to cause issues " +
    "in certain cases" do
  it "should work" do
    expect(true).to be true
  end
end

# block as argument inside parentheses — end aligns with method inside parens
expect(arr.all? do |o|
         o.valid?
       end)

# multiline with || continuation: do on second line
a ||
  items.each do |x|
  process(x)
end

# multiline with && continuation: do on second line
(value.is_a? Array) &&
  value.all? do |subvalue|
  type_check(subvalue)
end
