// Stands in for claude in the bridge's entry-point test: reports what it was given, then exits with 3.
process.stdout.write(JSON.stringify({ args: process.argv.slice(2), host: process.env.APOLLO_HOST }) + '\n')
process.exitCode = 3
