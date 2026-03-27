puts $1
     ^^ Style/PerlBackrefs: Prefer `Regexp.last_match(1)` over `$1`.

$9
^^ Style/PerlBackrefs: Prefer `Regexp.last_match(9)` over `$9`.

$&
^^ Style/PerlBackrefs: Prefer `Regexp.last_match(0)` over `$&`.

module Namespace
  name = $POSTMATCH
         ^^^^^^^^^^ Style/PerlBackrefs: Prefer `::Regexp.last_match.post_match` over `$POSTMATCH`.

  id = $PREMATCH
       ^^^^^^^^^ Style/PerlBackrefs: Prefer `::Regexp.last_match.pre_match` over `$PREMATCH`.

  hashed_password_without_type = $POSTMATCH
                                 ^^^^^^^^^^ Style/PerlBackrefs: Prefer `::Regexp.last_match.post_match` over `$POSTMATCH`.

  $MATCH
  ^^^^^^ Style/PerlBackrefs: Prefer `::Regexp.last_match(0)` over `$MATCH`.

  content = $POSTMATCH
            ^^^^^^^^^^ Style/PerlBackrefs: Prefer `::Regexp.last_match.post_match` over `$POSTMATCH`.

  raw_record = $POSTMATCH
               ^^^^^^^^^^ Style/PerlBackrefs: Prefer `::Regexp.last_match.post_match` over `$POSTMATCH`.

  raw_record = $POSTMATCH
               ^^^^^^^^^^ Style/PerlBackrefs: Prefer `::Regexp.last_match.post_match` over `$POSTMATCH`.

  raw_action = $POSTMATCH
               ^^^^^^^^^^ Style/PerlBackrefs: Prefer `::Regexp.last_match.post_match` over `$POSTMATCH`.
end
