foo(
        1
        ^^^ Layout/FirstArgumentIndentation: Indent the first argument one step more than the start of the previous line.
)
bar(
    2
    ^^ Layout/FirstArgumentIndentation: Indent the first argument one step more than the start of the previous line.
)
baz(
          3
          ^^^ Layout/FirstArgumentIndentation: Indent the first argument one step more than the start of the previous line.
)

# super() with wrong indentation
super(
        serializer: Serializer,
        ^^^^^^^^^^^^^^^^^^^^^^^^^^^ Layout/FirstArgumentIndentation: Indent the first argument one step more than the start of the previous line.
        host: host,
        port: port.to_i
)

# Non-parenthesized call with backslash continuation — first arg on next line
output = Whenever.cron \
    <<-file
    ^^^^ Layout/FirstArgumentIndentation: Indent the first argument one step more than the start of the previous line.
  set :job_template, nil
  every "weekday" do
    command "blahblah"
  end
file

# Another backslash continuation pattern
expect(subject.attributes).to eq \
    'alg' => 'test',
    ^^^^^ Layout/FirstArgumentIndentation: Indent the first argument one step more than the start of the previous line.
    'sub' => 'alice'

# Backslash continuation with wrong indent
assert_equal \
    "some long string value here",
    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Layout/FirstArgumentIndentation: Indent the first argument one step more than the start of the previous line.
  new_command.result.join(" ")

# Method call inside heredoc interpolation with wrong indentation
content = <<~HTML
  #{builder.attachment(
      :image,
      ^^ Layout/FirstArgumentIndentation: Indent the first argument one step more than the start of the previous line.
      titled: true
  )}
HTML

# Tab-indented code with wrong indentation (3 tabs instead of expected 4)
		loader.inflector.inflect(
			"csv" => "CSV",
			^^^^^^^^^^^^ Layout/FirstArgumentIndentation: Indent the first argument one step more than the start of the previous line.
			"svg" => "SVG"
		)

# Dotted operator call inside a block — should still be checked
foo.where { Sequel.|(
             { :level_enum__value => SERIES_LEVELS },
             ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Layout/FirstArgumentIndentation: Indent the first argument one step more than the start of the previous line.
             Sequel.&({ :level_enum__value => "otherlevel" },
                      { Sequel.function(:lower, :other_level) => OTHERLEVEL_SERIES_LEVELS })
) }

# Dotted operator call as an inner argument — message should use the base range
foo.filter(Sequel.|(
      Sequel.~(:agent_person_id => nil),
      ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Layout/FirstArgumentIndentation: Indent the first argument one step more than `Sequel.|(`.
      Sequel.~(:agent_family_id => nil)
))

      expect(WashOut::Dispatcher.deep_select(
        {
        ^ Layout/FirstArgumentIndentation: Indent the first argument one step more than `WashOut::Dispatcher.deep_select(`.
          k: {:@id => 5, x: :y},
          k2: {:@id => 6, n: :m}
        }, &blk)).to eq [{:@id => 5, x: :y}, {:@id => 6, n: :m}]

      expect(WashOut::Dispatcher.deep_select(
        {
        ^ Layout/FirstArgumentIndentation: Indent the first argument one step more than `WashOut::Dispatcher.deep_select(`.
          k: [{:@id => 5, x: :y}],
          k2: {:@id => 6, n: :m}
        }, &blk)).to contain_exactly({:@id => 5, x: :y}, {:@id => 6, n: :m})

      expect(WashOut::Dispatcher.deep_select(
        {
        ^ Layout/FirstArgumentIndentation: Indent the first argument one step more than `WashOut::Dispatcher.deep_select(`.
          k: [{:@id => 5, x: :y}],
          k2: [{:@id => 6, n: :m}]
        }, &blk)).to eq [{:@id => 5, x: :y}, {:@id => 6, n: :m}]
