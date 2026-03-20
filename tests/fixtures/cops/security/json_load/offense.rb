JSON.load(data)
     ^^^^ Security/JSONLoad: Prefer `JSON.parse` over `JSON.load`.
JSON.load("{}")
     ^^^^ Security/JSONLoad: Prefer `JSON.parse` over `JSON.load`.
::JSON.load(x)
       ^^^^ Security/JSONLoad: Prefer `JSON.parse` over `JSON.load`.
JSON.restore(arg)
     ^^^^^^^ Security/JSONLoad: Prefer `JSON.parse` over `JSON.restore`.
::JSON.restore(arg)
       ^^^^^^^ Security/JSONLoad: Prefer `JSON.parse` over `JSON.restore`.
JSON.load(arg, max_nesting: 1)
     ^^^^ Security/JSONLoad: Prefer `JSON.parse` over `JSON.load`.
