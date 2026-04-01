x =~ /\=/
      ^^ Style/RedundantRegexpEscape: Redundant escape of `=` in regexp.

x =~ /\:/
      ^^ Style/RedundantRegexpEscape: Redundant escape of `:` in regexp.

x =~ /\,/
      ^^ Style/RedundantRegexpEscape: Redundant escape of `,` in regexp.

# Inside character class: dot is redundant
x =~ /[\.]/
       ^^ Style/RedundantRegexpEscape: Redundant escape of `.` in regexp.
# Inside character class: plus is redundant
x =~ /[\+]/
       ^^ Style/RedundantRegexpEscape: Redundant escape of `+` in regexp.
# Escaped hyphen at end of character class is redundant
x =~ /[a-z0-9\-]/
             ^^ Style/RedundantRegexpEscape: Redundant escape of `-` in regexp.

let(:postmark_message_id_format) {/\w{8}\-\w{4}-\w{4}-\w{4}-\w{12}/}
                                        ^^ Style/RedundantRegexpEscape: Redundant escape of `-` in regexp.

let(:postmark_message_id_format) { /\w{8}\-\w{4}-\w{4}-\w{4}-\w{12}/ }
                                         ^^ Style/RedundantRegexpEscape: Redundant escape of `-` in regexp.

let(:postmark_message_id_format) { /\w{8}\-\w{4}-\w{4}-\w{4}-\w{12}/ }
                                         ^^ Style/RedundantRegexpEscape: Redundant escape of `-` in regexp.

/^\[\<assembly: #{attr_name}(.+)/
    ^^ Style/RedundantRegexpEscape: Redundant escape of `<` in regexp.

/^\<assembly: #{attr_name}(.+)/i  
  ^^ Style/RedundantRegexpEscape: Redundant escape of `<` in regexp.

Then /^I should have cucumber\-chef on my path$/ do
                             ^^ Style/RedundantRegexpEscape: Redundant escape of `-` in regexp.

Then /^I can get help about the cucumber\-chef binary on the command line$/ do
                                        ^^ Style/RedundantRegexpEscape: Redundant escape of `-` in regexp.

scheme = /https/
pattern = %r{
  #{scheme}
  (https?:\/\/)?
          ^^ Style/RedundantRegexpEscape: Redundant escape of `/` in regexp.
            ^^ Style/RedundantRegexpEscape: Redundant escape of `/` in regexp.
}x

chars = /a/
path_pattern = /(?:
  #{chars}
  [\.,]#{chars}
   ^^ Style/RedundantRegexpEscape: Redundant escape of `.` in regexp.
)/x
