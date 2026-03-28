foo(1, 2)
x = "a , b"
y = [1, 2, 3]
bar(a, b, c)
{a: 1, b: 2}
# Character literal escaped space before comma
x = {space: ?\ , tab: ?\t}
case c
when ?\ , ?\t, ?\r
  true
end

command './configure' \
        " --disable-lz4" \
        , env: env

buffer.insert(iter, "foo \
bar \
baz\n\n" ,
              :tags => ["rtl_quote"])
