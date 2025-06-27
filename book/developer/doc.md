# Documentation

## Generating Zisk documentation
The Zisk project is documented to help users and developers.

In order to generate the Zisk documentation, execute the cargo doc command.  We recommend using the
`--no-deps` flag to avoid generating documentation for all its external dependencies.

```sh
$ cargo doc --no-deps
```

This will generate a set of HTML files under the `target/doc` directory.

## Viewing Zisk documentation

The Zisk documentation can be visualized using a web browser and navigating to
`target/doc/cargo_zisk/`.

If you are working with Zisk in a remote server (typical setup during development) then first you
must export the local files through HTTP, for example using an HTTP proxy:

```sh
$ python3 -m http.server --bind 0.0.0.0 8000
```

Now, you can browse to the server:

http://<IP>:8000/target/doc/cargo_zisk/

## Adding content

Some basic hints:
* Only public modules and public elements will appear in the cargo documentation
* In particular, remember to make documented modules public in lib.rs (`pub mod <module>;`)
* Documentation for a public module must start in the first line in the module file, starting with
`//! ...`
* Documentation for a public element must be placed right before it, starting with `/// ...`
* Wrap code with triple spike: `//! \`\`\``
* To avoid cargo doc to compile the code, use `text` after the triple spike: `//! \`\`\`text`