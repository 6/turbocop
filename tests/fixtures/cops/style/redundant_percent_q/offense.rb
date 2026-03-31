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

%Q|
^ Style/RedundantPercentQ: Use `%Q` only for strings that contain both single quotes and double quotes, or for dynamic strings that contain double quotes.
  hostname: serveme.tf
|

%Q{
^ Style/RedundantPercentQ: Use `%Q` only for strings that contain both single quotes and double quotes, or for dynamic strings that contain double quotes.
  <script type="text/javascript">
}

%Q({"name": "foo", "values": {"a": "1", "b": "2", "c": "3"}})
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/RedundantPercentQ: Use `%Q` only for strings that contain both single quotes and double quotes, or for dynamic strings that contain double quotes.

%Q[{"mimebundle": "json"}]
^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/RedundantPercentQ: Use `%Q` only for strings that contain both single quotes and double quotes, or for dynamic strings that contain double quotes.

%Q(He said "hello" to me)
^^^^^^^^^^^^^^^^^^^^^^^^^ Style/RedundantPercentQ: Use `%Q` only for strings that contain both single quotes and double quotes, or for dynamic strings that contain double quotes.

%Q(<div class="action-markdown"> <h1>Title</h1> </div>)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/RedundantPercentQ: Use `%Q` only for strings that contain both single quotes and double quotes, or for dynamic strings that contain double quotes.
