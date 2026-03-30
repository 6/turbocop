%q(hi)
^^^^^^ Style/RedundantPercentQ: Use `%q` only for strings that contain both single quotes and double quotes.

%q('hi')
^^^^^^^^ Style/RedundantPercentQ: Use `%q` only for strings that contain both single quotes and double quotes.

%q("hi")
^^^^^^^^ Style/RedundantPercentQ: Use `%q` only for strings that contain both single quotes and double quotes.

%Q{#{foo} bar}
^^^^^^^^^^^^^^ Style/RedundantPercentQ: Use `%Q` only for strings that contain both single quotes and double quotes, or for dynamic strings that contain double quotes.

%Q|#{comment_singleline_token} #{string_data}|
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/RedundantPercentQ: Use `%Q` only for strings that contain both single quotes and double quotes, or for dynamic strings that contain double quotes.

%Q(hello world)
^^^^^^^^^^^^^^^ Style/RedundantPercentQ: Use `%Q` only for strings that contain both single quotes and double quotes, or for dynamic strings that contain double quotes.

x = %Q{#{a} #{b}}
    ^^^^^^^^^^^^^^ Style/RedundantPercentQ: Use `%Q` only for strings that contain both single quotes and double quotes, or for dynamic strings that contain double quotes.
