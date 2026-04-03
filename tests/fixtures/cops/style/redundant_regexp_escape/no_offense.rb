x =~ /\./
x =~ /\d+/
x =~ /\[foo\]/
x =~ /\\/
x =~ /foo/
y = 'hello'
# Escape hyphen in the middle of char class is meaningful
x =~ /[\s\-a]/
# Escape sequences in char class are meaningful
x =~ /[\w\d\s]/
# Escape bracket inside char class is meaningful
x =~ /[\]]/
# RuboCop keeps escaped hyphen immediately after `[^`
x =~ /[^\-^<]+/
# POSIX character classes keep a following escaped hyphen meaningful
x =~ /[[:alnum:]\-_]+/
# Escapes after `#` preserve interpolation sigils
x =~ /[#\$not_gvar]/
# Escaping delimiter characters in %r(...) is not redundant
x =~ %r(\A[^\(]*time)i
x =~ %r(foo\(bar\))
x =~ %r{foo\{bar\}}
# Backslash-newline is a regexp line continuation, not a redundant escape
x =~ /a\
b/
# Line continuation inside a character class is also allowed
x =~ /[a\
b]/
# Real-world multiline token regexp from the corpus
BEG_REGEXP = /\G(?:\
(?# 1:  SPACE   )( +)|\
(?# 2:  NIL     )(NIL))/
# Free-spacing comments are ignored
x = /foo # redundant unless commented: \-/x
# /e and /s suppress this cop like RuboCop
x =~ /\-/e
x =~ /\-/s
# RuboCop only reports interpolated block-call regexps up to the first interpolation
rule %r{(#{complex_id})(#{ws}*)([\{\(])}mx do |m|
end
# In /x regexps, RuboCop drops the whole regexp when blanking interpolation
# leaves a branch-start quantifier for regexp_parser
URIREGEX[:valid_url_path_chars] = /(?:
  #{URIREGEX[:wikipedia_disambiguation]}|
  @#{URIREGEX[:valid_general_url_path_chars]}+\/|
  [\.,]#{URIREGEX[:valid_general_url_path_chars]}+|
  #{URIREGEX[:valid_general_url_path_chars]}+
)/ix

URIREGEX[:valid_url] = %r{
      (
        (#{URIREGEX[:valid_preceding_chars]})
        (
          (https?:\/\/)?
          (#{URIREGEX[:valid_domain]})
          (/
            (?:
              #{URIREGEX[:valid_url_path_chars]}+#{URIREGEX[:valid_url_path_ending_chars]}|
              #{URIREGEX[:valid_url_path_chars]}+#{URIREGEX[:valid_url_path_ending_chars]}?|
              #{URIREGEX[:valid_url_path_ending_chars]}
            )?
          )?
          (\?#{URIREGEX[:valid_url_query_chars]}*#{URIREGEX[:valid_url_query_ending_chars]})?
        )
      )
    }iox
