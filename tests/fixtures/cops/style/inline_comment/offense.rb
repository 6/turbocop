two = 1 + 1 # A trailing inline comment
            ^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/InlineComment: Avoid trailing inline comments.
x = 42 # meaning of life
       ^^^^^^^^^^^^^^^^^^^ Style/InlineComment: Avoid trailing inline comments.
foo(bar) # call foo
         ^^^^^^^^^^ Style/InlineComment: Avoid trailing inline comments.

=begin
^ Style/InlineComment: Avoid trailing inline comments.
=end

value = 1
=begin
^ Style/InlineComment: Avoid trailing inline comments.
=end
