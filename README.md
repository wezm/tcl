Tiny Command Language
=====================

The Tiny Command Language is a little command language inspired by the
[Tool Command Language (Tcl)](https://www.tcl-lang.org/). It aims to
serve as a simple extension language for Rust programs and was written as part
of the Untitled Ports Project.

**NOTE:** Tcl is still in development and some of the following is
aspirational.

Tcl is command oriented, similar to a UNIX shell. Commands are comprised of
words. The first word is the command name the remainder are arguments to that
command. Words are whitespace (space or tab) separated. Double quotes may be
used to create words with spaces or newlines in them. Words may contain nearly
any Unicode character.

```tcl
puts "Hello, world"
```

Commands are newline or `;` terminated. `{}` can be used to allow a command to be
spread over multiple lines.

```tcl
user "Example User" {
  uid 1000
  gid 1000
  shell /bin/zsh
}
```

Is equivalent to:

```tcl
user "Example User" uid 1000 gid 1000 shell /bin/zsh
```

The built-in command `set` can be used to set a variable. Variable substitution
in a word can be performed with `$var_name` or `${var_name}`.

E.g.

```tcl
set example world
puts "Hello, $example"
```

Command substitution can be performed with `[]`. The result of the inner
command will be used in its place.

E.g. `puts [ + 1 2 ]` would print `3`.
