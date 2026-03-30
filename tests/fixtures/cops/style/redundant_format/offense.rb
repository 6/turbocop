format('foo')
^^^^^^^^^^^^^ Style/RedundantFormat: Use `'foo'` directly instead of `format`.

sprintf('bar')
^^^^^^^^^^^^^^ Style/RedundantFormat: Use `'bar'` directly instead of `sprintf`.

Kernel.format('baz')
^^^^^^^^^^^^^^^^^^^^ Style/RedundantFormat: Use `'baz'` directly instead of `format`.

format(FORMAT)
^^^^^^^^^^^^^^ Style/RedundantFormat: Use `FORMAT` directly instead of `format`.

sprintf(MSG)
^^^^^^^^^^^^ Style/RedundantFormat: Use `MSG` directly instead of `sprintf`.

format(Foo::BAR)
^^^^^^^^^^^^^^^^ Style/RedundantFormat: Use `Foo::BAR` directly instead of `format`.

format('%s %s', 'foo', 'bar')
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/RedundantFormat: Use `'foo bar'` directly instead of `format`.

sprintf('%-10s', 'foo')
^^^^^^^^^^^^^^^^^^^^^^^ Style/RedundantFormat: Use `'foo       '` directly instead of `sprintf`.

format('%d', 5)
^^^^^^^^^^^^^^^ Style/RedundantFormat: Use `'5'` directly instead of `format`.

format('%s', 'hello')
^^^^^^^^^^^^^^^^^^^^^ Style/RedundantFormat: Use `'hello'` directly instead of `format`.

format('%s', :foo)
^^^^^^^^^^^^^^^^^^ Style/RedundantFormat: Use `'foo'` directly instead of `format`.

expect(@parameter.format("hello %s", "world")).to eq("hello world")
       ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/RedundantFormat: Use `"hello world"` directly instead of `format`.

expect(@parameter.format("hello %s", "world")).to eq("hello [redacted]")
       ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/RedundantFormat: Use `"hello world"` directly instead of `format`.

format 'text/latex' do |obj|
^ Style/RedundantFormat: Use `'text/latex'` directly instead of `format`.

format 'text/latex' do |obj|
^ Style/RedundantFormat: Use `'text/latex'` directly instead of `format`.

format 'text/html' do |obj|
^ Style/RedundantFormat: Use `'text/html'` directly instead of `format`.

format 'text/latex' do |obj|
^ Style/RedundantFormat: Use `'text/latex'` directly instead of `format`.

format 'text/html' do |obj|
^ Style/RedundantFormat: Use `'text/html'` directly instead of `format`.

format 'text/latex' do |obj|
^ Style/RedundantFormat: Use `'text/latex'` directly instead of `format`.

puts format('%-38<form_uuid>s %-25<file_name>s %-12<s3_status>s %-20<created_at>s %<issue>s',
     ^ Style/RedundantFormat: Use `'FORM_UUID                              FILE_NAME                 S3_STATUS    CREATED_AT           ISSUE'` directly instead of `format`.
            form_uuid: 'FORM_UUID', file_name: 'FILE_NAME', s3_status: 'S3_STATUS',
            created_at: 'CREATED_AT', issue: 'ISSUE')

puts format('%-38<form_uuid>s %-20<timestamp>s %<error>s',
     ^ Style/RedundantFormat: Use `'FORM_UUID                              TIMESTAMP            ERROR'` directly instead of `format`.
            form_uuid: 'FORM_UUID', timestamp: 'TIMESTAMP', error: 'ERROR')

format("Your card has been removed (number: %s)", "x-#{default_card.last_digits}")
^ Style/RedundantFormat: Use `"Your card has been removed (number: x-#{default_card.last_digits})"` directly instead of `format`.

PDK.logger.warn_once format('%{varname} is not supported by PDK.', varname: "#{gem}_GEM_VERSION") if PDK::Util::Env["#{gem}_GEM_VERSION"]
                     ^ Style/RedundantFormat: Use `"#{gem}_GEM_VERSION is not supported by PDK."` directly instead of `format`.

PDK::Report.default_target.puts(format("\n%{summary}\n\n", summary: "#{summary_to_print.join(', ')}."))
                                ^ Style/RedundantFormat: Use `"\n#{summary_to_print.join(', ')}.\n\n"` directly instead of `format`.
