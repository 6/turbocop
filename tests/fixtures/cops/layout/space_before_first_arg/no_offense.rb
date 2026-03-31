foo x
bar 1, 2
baz "hello"
foo(x)
bar(1, 2)
something.method x

# Aligned extra spaces are allowed (AllowForAlignment: true default)
# The arguments :full_name, :password, :zip_code etc align vertically
form.inline_input   :full_name,     as: :string
form.disabled_input :password,      as: :passwd
form.masked_input   :zip_code,      as: :string
form.masked_input   :email_address, as: :email
form.masked_input   :phone_number,  as: :tel

# Operator methods should not be flagged
2**128
x + 1
a << b
arr[0]
x != y

# Setter methods should not be flagged
something.x = y

# Multiple spaces containing line break
something \
  x

# Method call across lines
something x,
          y

# Alignment with same-indentation line separated by differently-indented lines.
# The has_many/has_one calls align their first argument, but continuation lines
# in between have different indentation. RuboCop's second pass (same-indent
# filter) finds the alignment even though the nearest non-blank lines don't match.
has_many    :foo, -> { where(active: true) },
                  as:         :addressable,
                  class_name: 'Address'
has_one     :bar, as: :addressable,
                  class_name: 'Address'

# Continuation-line sends are not checked for extra spacing when the send
# itself starts on a previous line.
Treat::Entities::Entity.call_worker(
'$'.to_entity, :tag, :lingua,
Treat::Workers::Lexicalizers::Taggers, {}).
should  eql '$'.tag(:lingua)

# Alignment should use character columns, not byte columns.
expect(JsRegex.new(/a/, options: 'f').options).to      eq('')
expect(JsRegex.new(/a/, options: 'fLüYz').options).to  eq('')
expect(JsRegex.new(/a/, options: '').options).to       eq('')

expect(RomanChord.new('iiio7', key: key).quality.name).to   eq 'dim7'
expect(RomanChord.new('ivø', key: key).quality.name).to     eq 'm7b5'
expect(RomanChord.new('VIIm7b5', key: key).quality.name).to eq 'm7b5'
