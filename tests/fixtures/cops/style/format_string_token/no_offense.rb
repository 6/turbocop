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
# Unannotated tokens in interpolated format strings are NOT flagged
# because str parts inside dstr don't have format context in RuboCop
l = format("#{prefix} %s %s", a, b)
m = sprintf("#{prefix} %d %d", a, b)
# Unannotated in heredoc used as format string
n = format(<<~FMT, a, b)
  %s
  %s
FMT
# Unannotated tokens in non-format-context string
o = "contains %s and %d tokens"
# Strings inside backtick (xstr) context are skipped
p = `curl -w '%{http_code}' http://example.com`
q = `echo %{name} %s`
# Heredoc used with % operator: unannotated tokens not flagged
# (RuboCop parses heredocs as dstr, so str parts lose format context)
r = <<-TEXT % [name, target, score, result, elapsed, verify]
  block %s
  target: %s
  data: '%s' + %s (nonce)
  found: %s
  time: %f
  verify: %f
TEXT
# Single-quoted multiline heredoc with % operator: unannotated tokens still not flagged
# because Parser treats multiline heredocs as dstr in this context.
r2 = <<-'TEXT' % [name, target]
  block %s
  target: %s
TEXT
# Multi-line %[] string literal with % operator: unannotated tokens not flagged
# (RuboCop's Parser gem produces dstr for multi-line strings, so parts lose format context)
s = %[service %s
  started at %s] % [svc, time]
# String with interpolation in format specifier: %#{var}s is not a token
t = format("%#{padding}s: %s", prefix, message)
u = sprintf("| %-#{width}s | %-#{offset}s |", key, value)
# Adjacent string literals passed to format are parsed as dstr parts, so
# unannotated tokens inside each part are not in the immediate format context.
v = format(
  '  Executed %d times in %.1fms (%.1f/%.1f/%.1fms min/median/max),' \
  " returning %d rows(%d bytes):\n"\
  "    %s\n" \
  "    First exec was: %s\n" \
  '    %s',
  queries, total_time, min_time, median_time, max_time, rows, bytes, name, sql, backtrace
)
# Continuation-line template tokens after escaped newlines are skipped because
# RuboCop's Parser-backed str node value drops the trailing `%{...}` here.
w = _("Status update
  details: \n\n%{explanation}") % { explanation: reason }
