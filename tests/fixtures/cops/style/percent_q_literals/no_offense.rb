%q(hello world)
%q[foo bar]
'hello world'
"hello world"
"hello #{name}"
%Q(hello #{name})
%Q(hello\tworld)
%Q{line one\nline two}
%Q(escaped\\backslash)
%Q[null\0byte]
%Q(unicode\u0041char)
# Multiline %Q strings are not flagged (Parser gem sees them as dstr, not str)
execute(%Q{
  UPDATE projects
    SET finished = true
    WHERE finished = false;
})
%Q(
  hello world
)
%Q[
  multiline
  content
  here
]
