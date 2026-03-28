'10'.to_i
^^^^^^^^^ Lint/NumberConversion: Replace unsafe number conversion with number class parsing, instead of using `'10'.to_i`, use stricter `Integer('10', 10)`.
'10.2'.to_f
^^^^^^^^^^^ Lint/NumberConversion: Replace unsafe number conversion with number class parsing, instead of using `'10.2'.to_f`, use stricter `Float('10.2')`.
'1/3'.to_r
^^^^^^^^^^ Lint/NumberConversion: Replace unsafe number conversion with number class parsing, instead of using `'1/3'.to_r`, use stricter `Rational('1/3')`.
# Safe navigation should still be flagged
"10"&.to_i
^^^^^^^^^^ Lint/NumberConversion: Replace unsafe number conversion with number class parsing, instead of using `"10".to_i`, use stricter `Integer("10", 10)`.
# Symbol form: map(&:to_i)
"1,2,3".split(',').map(&:to_i)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/NumberConversion: Replace unsafe number conversion with number class parsing, instead of using `&:to_i`, use stricter `{ |i| Integer(i, 10) }`.
# Symbol form: try(:to_f)
"foo".try(:to_f)
^^^^^^^^^^^^^^^^ Lint/NumberConversion: Replace unsafe number conversion with number class parsing, instead of using `:to_f`, use stricter `{ |i| Float(i) }`.
# Symbol form: send(:to_c)
"foo".send(:to_c)
^^^^^^^^^^^^^^^^^ Lint/NumberConversion: Replace unsafe number conversion with number class parsing, instead of using `:to_c`, use stricter `{ |i| Complex(i) }`.
# Symbol form without parentheses
"1,2,3".split(',').map &:to_i
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/NumberConversion: Replace unsafe number conversion with number class parsing, instead of using `&:to_i`, use stricter `{ |i| Integer(i, 10) }`.
# Symbol form with safe navigation
"1,2,3".split(',')&.map(&:to_i)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/NumberConversion: Replace unsafe number conversion with number class parsing, instead of using `&:to_i`, use stricter `{ |i| Integer(i, 10) }`.
# Bare symbol form without explicit receiver (implicit self)
map(&:to_i)
^^^^^^^^^^^ Lint/NumberConversion: Replace unsafe number conversion with number class parsing, instead of using `&:to_i`, use stricter `{ |i| Integer(i, 10) }`.
try(:to_f)
^^^^^^^^^^ Lint/NumberConversion: Replace unsafe number conversion with number class parsing, instead of using `:to_f`, use stricter `{ |i| Float(i) }`.
send(:to_c)
^^^^^^^^^^^ Lint/NumberConversion: Replace unsafe number conversion with number class parsing, instead of using `:to_c`, use stricter `{ |i| Complex(i) }`.
# Qualified constant (Core::Utils::Time) does NOT match "Time" in IgnoredClasses
Core::Utils::Time.now.to_i
^^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/NumberConversion: Replace unsafe number conversion with number class parsing, instead of using `Core::Utils::Time.now.to_i`, use stricter `Integer(Core::Utils::Time.now, 10)`.
Faker::Time.backward(days: 365).to_i
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/NumberConversion: Replace unsafe number conversion with number class parsing, instead of using `Faker::Time.backward(days: 365).to_i`, use stricter `Integer(Faker::Time.backward(days: 365), 10)`.
# Symbol argument with regular block (not block argument) should still be flagged
receive(:to_i) { 1 }
^^^^^^^^^^^^^^^^^^^^^ Lint/NumberConversion: Replace unsafe number conversion with number class parsing, instead of using `:to_i`, use stricter `{ |i| Integer(i, 10) }`.

retry_after = e.response.headers[:retry_after]&.to_i ||= (0.5 * (retry_count + 1))
              ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/NumberConversion: Replace unsafe number conversion with number class parsing, instead of using `e.response.headers[:retry_after].to_i`, use stricter `Integer(e.response.headers[:retry_after], 10)`.

retry_after = e.response.headers[:retry_after]&.to_i ||= (0.5 * (retry_count + 1))
              ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/NumberConversion: Replace unsafe number conversion with number class parsing, instead of using `e.response.headers[:retry_after].to_i`, use stricter `Integer(e.response.headers[:retry_after], 10)`.

retry_after = e.response.headers[:retry_after]&.to_i ||= (0.5 * (retry_count + 1))
              ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/NumberConversion: Replace unsafe number conversion with number class parsing, instead of using `e.response.headers[:retry_after].to_i`, use stricter `Integer(e.response.headers[:retry_after], 10)`.

port = opts[:port].to_i ||= 8888
       ^^^^^^^^^^^^^^^^ Lint/NumberConversion: Replace unsafe number conversion with number class parsing, instead of using `opts[:port].to_i`, use stricter `Integer(opts[:port], 10)`.

f = flow_data[i]&.second.to_f
    ^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/NumberConversion: Replace unsafe number conversion with number class parsing, instead of using `flow_data[i]&.second.to_f`, use stricter `Float(flow_data[i]&.second)`.

f = flow_data[i]&.second.to_f
    ^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/NumberConversion: Replace unsafe number conversion with number class parsing, instead of using `flow_data[i]&.second.to_f`, use stricter `Float(flow_data[i]&.second)`.
