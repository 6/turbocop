#missing space
^ Layout/LeadingCommentSpace: Missing space after `#`.
x = 1
#bad
^ Layout/LeadingCommentSpace: Missing space after `#`.
y = 2
#another bad comment
^ Layout/LeadingCommentSpace: Missing space after `#`.

##patterns += patterns.collect(&:to_s)
^ Layout/LeadingCommentSpace: Missing space after `#`.

##$FUNCTOR_EXCEPTIONS ||= [:binding]
^ Layout/LeadingCommentSpace: Missing space after `#`.
