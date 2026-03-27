x =~ /[x]/
      ^^^ Style/RedundantRegexpCharacterClass: Redundant single-element character class, `[x]` can be replaced with `x`.

x =~ /[\d]/
      ^^^^ Style/RedundantRegexpCharacterClass: Redundant single-element character class, `[\d]` can be replaced with `\d`.

x =~ /[a]b[c]d/
      ^^^ Style/RedundantRegexpCharacterClass: Redundant single-element character class, `[a]` can be replaced with `a`.
          ^^^ Style/RedundantRegexpCharacterClass: Redundant single-element character class, `[c]` can be replaced with `c`.

x =~ /([a])/
       ^^^ Style/RedundantRegexpCharacterClass: Redundant single-element character class, `[a]` can be replaced with `a`.

!!(/\A(https|http)\:\/\/[a-zA-Z0-9][a-zA-Z0-9\-]*\.#{Regexp.quote(options[:myshopify_domain])}[\/]?\z/ =~ options[:client_options][:site])
# nitrocop-expect: 9:94 Style/RedundantRegexpCharacterClass: Redundant single-element character class, `[\/]` can be replaced with `\/`.

str = str.gsub(/[髙]/, "高")
                ^^^ Style/RedundantRegexpCharacterClass: Redundant single-element character class, `[髙]` can be replaced with `髙`.

str = str.gsub(/[埼]/, "崎")
                ^^^ Style/RedundantRegexpCharacterClass: Redundant single-element character class, `[埼]` can be replaced with `埼`.

include_examples 'parse', /a(b(\d|[ef-g[h]]))/,
# nitrocop-expect: 15:39 Style/RedundantRegexpCharacterClass: Redundant single-element character class, `[h]` can be replaced with `h`.

match = line.match(/#{grep_word}=.*\{[\w]+\s*([\w\.\:\!]+\s*)*\/*([\w\.]+)*/)
# nitrocop-expect: 17:37 Style/RedundantRegexpCharacterClass: Redundant single-element character class, `[\w]` can be replaced with `\w`.

current_entry = Array(l.sub(/^#{CHANGELOG_TAG}:[\s]*/, ""))
# nitrocop-expect: 19:47 Style/RedundantRegexpCharacterClass: Redundant single-element character class, `[\s]` can be replaced with `\s`.

/<#{tag}([\s]+([-[:word:]]+)[\s]*\=\s*\"([^\"]*)\")*\s*>.*<\s*\/#{tag}\s*>/
         ^^^^ Style/RedundantRegexpCharacterClass: Redundant single-element character class, `[\s]` can be replaced with `\s`.
                            ^^^^ Style/RedundantRegexpCharacterClass: Redundant single-element character class, `[\s]` can be replaced with `\s`.
