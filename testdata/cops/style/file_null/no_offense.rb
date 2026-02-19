path = File::NULL
x = ""
'the null devices are /dev/null on Unix and NUL on Windows'
"/dev/null is used on Unix"
"redirect to NUL on Windows"
File.open(File::NULL)

# Inside a hash pair — acceptable per RuboCop
exec('server', out: '/dev/null')
{ unix: "/dev/null", windows: "nul" }

# Inside an array — acceptable per RuboCop
null_devices = %w[/dev/null nul]
