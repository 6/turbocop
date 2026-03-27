folders = %x(find . -type d).split
          ^^^^^^^^^^^^^^^^^^^ Style/CommandLiteral: Use backticks around command string.

result = %x(ls -la)
         ^^^^^^^^^^ Style/CommandLiteral: Use backticks around command string.

output = %x(echo hello)
         ^^^^^^^^^^^^^^ Style/CommandLiteral: Use backticks around command string.

lines = `git log \`git describe --tags --abbrev=0\`..HEAD --oneline`.split("\n")
        ^ Style/CommandLiteral: Use `%x` around command string.

v = `cygpath '#{`regtool get #{args.join(' ')}`.strip}'`.strip
    ^ Style/CommandLiteral: Use `%x` around command string.

`\``
^ Style/CommandLiteral: Use `%x` around command string.

return `self[#{rng.rand(`self.length`)}]` unless count
       ^ Style/CommandLiteral: Use `%x` around command string.

`for (const c of self) #{yield `c.codePointAt(0)`}`
^ Style/CommandLiteral: Use `%x` around command string.

`for (const cluster of clusters) #{yield `$str(cluster.segment, self.encoding)`}`
^ Style/CommandLiteral: Use `%x` around command string.

self.proc = `function(status){ #{KernelExit.status = `status`} }`
            ^ Style/CommandLiteral: Use `%x` around command string.

`Opal.exit = function(status) { #{received_status = `status`} }`
^ Style/CommandLiteral: Use `%x` around command string.

`
^ Style/CommandLiteral: Use `%x` around command string.
  callback(#{block.call(`realpath`)})
`
