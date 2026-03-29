'abc'.strip

'abc'.strip

str.strip

strip.gsub(/\{\{([^\}]+)\}\}/) { |special|

strip

strip.gsub(/\{\{([^\}]+)\}\}/) { |special|

def cleanup_entry(line)
  line[1..-1].gsub(/\[\[([^\|\]]+)\|([^\]]+)\]\]/) { |link|
    $2
  }.
    gsub(/''/, "'").
    gsub(/\]\]/, "").
    gsub(/\[\[/, "").
    gsub(/\&ndash;/, "-").
    gsub(/ +/, ' ').
    strip.gsub(/\{\{([^\}]+)\}\}/) { |special|
    stuff = $1.split("|")
  }
end
