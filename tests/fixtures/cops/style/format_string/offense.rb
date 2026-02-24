sprintf('%s', 'hello')
^^^^^^^ Style/FormatString: Favor `format` over `sprintf`.
'%s' % 'hello'
     ^ Style/FormatString: Favor `format` over `String#%`.
"%d items" % count
           ^ Style/FormatString: Favor `format` over `String#%`.
msg % [arg1, arg2]
    ^ Style/FormatString: Favor `format` over `String#%`.
template % { name: value }
         ^ Style/FormatString: Favor `format` over `String#%`.
foo.bar % [1, 2, 3]
        ^ Style/FormatString: Favor `format` over `String#%`.
TEMPLATE % [a, b]
         ^ Style/FormatString: Favor `format` over `String#%`.
