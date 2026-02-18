path = File::NULL
x = ""
'the null devices are /dev/null on Unix and NUL on Windows'
"/dev/null is used on Unix"
"redirect to NUL on Windows"
File.open(File::NULL)
