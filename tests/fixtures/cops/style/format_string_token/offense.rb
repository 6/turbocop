x = '%{name} is %{age}'
     ^^^^^^^ Style/FormatStringToken: Prefer annotated tokens (like `%<foo>s`) over template tokens (like `%{foo}`).
                ^^^^^^ Style/FormatStringToken: Prefer annotated tokens (like `%<foo>s`) over template tokens (like `%{foo}`).
y = format('%s %s %d', a, b, c)
            ^^ Style/FormatStringToken: Prefer annotated tokens (like `%<foo>s`) over unannotated tokens (like `%s`).
               ^^ Style/FormatStringToken: Prefer annotated tokens (like `%<foo>s`) over unannotated tokens (like `%s`).
                  ^^ Style/FormatStringToken: Prefer annotated tokens (like `%<foo>s`) over unannotated tokens (like `%s`).
z = '%{greeting} %{target}'
     ^^^^^^^^^^ Style/FormatStringToken: Prefer annotated tokens (like `%<foo>s`) over template tokens (like `%{foo}`).
                 ^^^^^^^^^ Style/FormatStringToken: Prefer annotated tokens (like `%<foo>s`) over template tokens (like `%{foo}`).
w = sprintf('%s %s', a, b)
             ^^ Style/FormatStringToken: Prefer annotated tokens (like `%<foo>s`) over unannotated tokens (like `%s`).
                ^^ Style/FormatStringToken: Prefer annotated tokens (like `%<foo>s`) over unannotated tokens (like `%s`).
v = <<~HEREDOC
  hello %{name}
        ^^^^^^^ Style/FormatStringToken: Prefer annotated tokens (like `%<foo>s`) over template tokens (like `%{foo}`).
  world %{age}
        ^^^^^^ Style/FormatStringToken: Prefer annotated tokens (like `%<foo>s`) over template tokens (like `%{foo}`).
HEREDOC
# Template tokens in regular strings used with redirect
a1 = "admin/customize/watched_words/%{path}"
                                    ^^^^^^ Style/FormatStringToken: Prefer annotated tokens (like `%<foo>s`) over template tokens (like `%{foo}`).
a2 = "tag/%{tag_id}"
          ^^^^^^^^^ Style/FormatStringToken: Prefer annotated tokens (like `%<foo>s`) over template tokens (like `%{foo}`).
# Unannotated tokens in format context with % operator
a3 = "items/%s/%s...%s" % [file, ver1, ver2]
            ^^ Style/FormatStringToken: Prefer annotated tokens (like `%<foo>s`) over unannotated tokens (like `%s`).
               ^^ Style/FormatStringToken: Prefer annotated tokens (like `%<foo>s`) over unannotated tokens (like `%s`).
                    ^^ Style/FormatStringToken: Prefer annotated tokens (like `%<foo>s`) over unannotated tokens (like `%s`).

sql = <<-'SQL' % [columns, values]
  INSERT OR REPLACE INTO moz_cookies (%s) VALUES (%s)
                                      ^^ Style/FormatStringToken: Prefer annotated tokens (like `%<foo>s`) over unannotated tokens (like `%s`).
                                                  ^^ Style/FormatStringToken: Prefer annotated tokens (like `%<foo>s`) over unannotated tokens (like `%s`).
SQL

multi = _("Status %{url}
                  ^^^^^^ Style/FormatStringToken: Prefer annotated tokens (like `%<foo>s`) over template tokens (like `%{foo}`).
  approved") % { url: target_url }
