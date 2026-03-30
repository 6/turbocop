{ 'foo' => 1 }
  ^^^^^ Style/StringHashKeys: Prefer symbols instead of strings as hash keys.

{ 'bar' => 2, 'baz' => 3 }
  ^^^^^ Style/StringHashKeys: Prefer symbols instead of strings as hash keys.
              ^^^^^ Style/StringHashKeys: Prefer symbols instead of strings as hash keys.

x = { 'key' => 'value' }
      ^^^^^ Style/StringHashKeys: Prefer symbols instead of strings as hash keys.

# String keys in a hash that is the receiver of gsub (not an argument)
{ 'expiration' => time, 'conditions' => conds }.to_json.gsub("\n", "")
  ^^^^^^^^^^^^^ Style/StringHashKeys: Prefer symbols instead of strings as hash keys.
                        ^^^^^^^^^^^^ Style/StringHashKeys: Prefer symbols instead of strings as hash keys.

# String keys in a hash nested inside an array argument of IO.popen
IO.popen([{"FOO" => "bar"}, "ruby", "foo.rb"])
           ^^^^^ Style/StringHashKeys: Prefer symbols instead of strings as hash keys.

# String keys in a block inside spawn/system (not direct arg)
system("cmd") do
  x = { 'inner' => 1 }
        ^^^^^^^ Style/StringHashKeys: Prefer symbols instead of strings as hash keys.
end

# Non-identifier string keys are also flagged (RuboCop autocorrects to :"Content-Type" etc.)
{ "Content-Type" => "text/html" }
  ^^^^^^^^^^^^^^ Style/StringHashKeys: Prefer symbols instead of strings as hash keys.
{ "foo bar" => 1 }
  ^^^^^^^^^ Style/StringHashKeys: Prefer symbols instead of strings as hash keys.

h = { __FILE__ => [1] }
      ^^^^^^^^ Style/StringHashKeys: Prefer symbols instead of strings as hash keys.

expect(Coverage).to receive(:result).once.and_return(__FILE__ => [0, 1])
                                                     ^^^^^^^^ Style/StringHashKeys: Prefer symbols instead of strings as hash keys.
