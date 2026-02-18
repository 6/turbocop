x = '%<name>s is %<age>d'
y = '%s'
z = 'hello world'
a = '%%s'
b = '%<greeting>s %<target>s'
c = '%d'
d = '%c/%u |%b%i| %e'
e = "%b %d %l:%M%P"
g = '%s %s %d'
# Incomplete template token: %{ without closing }name
h = '%{'
i = ['%{', '}']
# Incomplete annotated token: %< without closing >
j = '%<'
# Interpolated string with %{ that doesn't form complete token
k = "%{#{keyword}}"
