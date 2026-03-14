foo(1,2)
     ^ Layout/SpaceAfterComma: Space missing after comma.
x = [1,2,3]
      ^ Layout/SpaceAfterComma: Space missing after comma.
        ^ Layout/SpaceAfterComma: Space missing after comma.
bar(a,b, c)
     ^ Layout/SpaceAfterComma: Space missing after comma.
raise "invalid at #{s[0,10].inspect}"
                       ^ Layout/SpaceAfterComma: Space missing after comma.
x = "result: #{obj.fetch(:a,default)}"
                           ^ Layout/SpaceAfterComma: Space missing after comma.
query = <<~SQL
  #{name.gsub(' ','')}
                 ^ Layout/SpaceAfterComma: Space missing after comma.
SQL
msg = <<~MSG
  #{records.map{|k,v| "#{k}=#{v}"}.join("\n")}
                  ^ Layout/SpaceAfterComma: Space missing after comma.
MSG
txt = <<~TXT
  #{w.message.to_s[0,40]}
                    ^ Layout/SpaceAfterComma: Space missing after comma.
TXT
