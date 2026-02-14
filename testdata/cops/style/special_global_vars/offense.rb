puts $0
     ^^ Style/SpecialGlobalVars: Prefer `$PROGRAM_NAME` over `$0`.

puts $!
     ^^ Style/SpecialGlobalVars: Prefer `$ERROR_INFO` over `$!`.

puts $$
     ^^ Style/SpecialGlobalVars: Prefer `$PROCESS_ID` over `$$`.

puts $?
     ^^ Style/SpecialGlobalVars: Prefer `$CHILD_STATUS` over `$?`.

puts $~
     ^^ Style/SpecialGlobalVars: Prefer `$LAST_MATCH_INFO` over `$~`.

puts $_
     ^^ Style/SpecialGlobalVars: Prefer `$LAST_READ_LINE` over `$_`.
