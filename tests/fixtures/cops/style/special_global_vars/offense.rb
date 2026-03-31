puts $0
     ^^ Style/SpecialGlobalVars: Prefer `$PROGRAM_NAME` over `$0`.

puts $!
     ^^ Style/SpecialGlobalVars: Prefer `$ERROR_INFO` over `$!`. Use `require 'English'` to access it.

puts $$
     ^^ Style/SpecialGlobalVars: Prefer `$PROCESS_ID` over `$$`. Use `require 'English'` to access it.

puts $?
     ^^ Style/SpecialGlobalVars: Prefer `$CHILD_STATUS` over `$?`. Use `require 'English'` to access it.

puts $~
     ^^ Style/SpecialGlobalVars: Prefer `$LAST_MATCH_INFO` over `$~`. Use `require 'English'` to access it.

puts $_
     ^^ Style/SpecialGlobalVars: Prefer `$LAST_READ_LINE` over `$_`. Use `require 'English'` to access it.

$: << File.expand_path('../lib', __FILE__)
^ Style/SpecialGlobalVars: Prefer `$LOAD_PATH` over `$:`.

$:.push File.expand_path("../lib", __FILE__)
^ Style/SpecialGlobalVars: Prefer `$LOAD_PATH` over `$:`.

$:.push File.expand_path("../lib", __FILE__)
^ Style/SpecialGlobalVars: Prefer `$LOAD_PATH` over `$:`.

$:.push File.expand_path("../lib", __FILE__)
^ Style/SpecialGlobalVars: Prefer `$LOAD_PATH` over `$:`.

$: << File.expand_path('../../lib', __FILE__)
^ Style/SpecialGlobalVars: Prefer `$LOAD_PATH` over `$:`.

$:.push File.expand_path('lib', __dir__)
^ Style/SpecialGlobalVars: Prefer `$LOAD_PATH` over `$:`.

$:.push File.expand_path('lib', __dir__)
^ Style/SpecialGlobalVars: Prefer `$LOAD_PATH` over `$:`.

$:.unshift File.expand_path('../../../lib', __dir__)
^ Style/SpecialGlobalVars: Prefer `$LOAD_PATH` over `$:`.
