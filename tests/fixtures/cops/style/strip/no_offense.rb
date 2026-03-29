'abc'.strip
'abc'.lstrip
'abc'.rstrip
str.strip
str.lstrip.downcase
str.rstrip.upcase

def cleanup_entry(line)
  line[1..-1].gsub(/\[\[([^\|\]]+)\|([^\]]+)\]\]/) { |link|
    $2
  }.
    gsub(/''/, "'").
    gsub(/\]\]/, "").
    gsub(/\[\[/, "").
    gsub(/\&ndash;/, "-").
    gsub(/ +/, ' ')
end
