''
"578.194.591.059"
"My IP is 192.168.1.1"
"2001:db8::1xyz"
"::"
'hello world'
# IPv4 with leading zeros (not valid per Ruby's Resolv::IPv4::Regex)
'01.02.03.04'
'001.002.003.004'
'1.2.3.04'
'192.168.001.001'
'10.0.0.01'
# IP inside interpolated string segments (no opening delimiter)
"before #{x} 127.0.0.1"
"#{prefix}10.0.0.1#{suffix}"
# Escape sequences that expand to IP-like content
"\x31.2.3.4"
# Triple colons and related patterns are NOT valid IPv6
':::'
':::A'
'::A:'
':::A:'
# IP address strings inside regexp interpolation (RuboCop ignores strings in regexps)
x =~ /^#{method('::1')}\b/
y =~ /#{addr('127.0.0.1')}/
# Zone ID without valid fe80 prefix is not a valid IP
'dead::beef%eth0'
# Zone ID with invalid format
'fe80::1%'
# Percent-literal strings (RuboCop's source[1...-1] garbles the content)
%q{10.33.33.33}
%(0.0.0.0)
