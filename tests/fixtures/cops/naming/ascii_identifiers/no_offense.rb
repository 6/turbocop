def resume
  nil
end

good_var = 1

def initialize
  @x = 1
end

x = "café"

# Comment with accénts

:symbol_ok

﻿require 'webmachine/configuration'

﻿puts 'foo'

# alias with explicit symbol notation — RuboCop checks tIDENTIFIER tokens,
# not tSYMBOL tokens. Explicit :symbol args are not identifiers.
alias :non_è_nullo? :esiste?
alias :è_un? :is_a?

# Method calls with non-ASCII chars and ? or ! suffix are tFID tokens,
# not tIDENTIFIER. RuboCop only checks tIDENTIFIER and tCONSTANT.
è_un_commento?
è_una_stringa?
non_è_nullo!

# Method definitions with ? or ! suffix are also tFID
def è_un_commento?
  false
end
