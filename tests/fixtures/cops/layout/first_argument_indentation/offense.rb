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

# Tab-indented code with wrong indentation (3 tabs instead of expected 4)
		loader.inflector.inflect(
			"csv" => "CSV",
			^^^^^^^^^^^^ Layout/FirstArgumentIndentation: Indent the first argument one step more than the start of the previous line.
			"svg" => "SVG"
		)
