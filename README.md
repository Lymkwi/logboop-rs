# LogBoop, a program to parse, split, and destroy rotated log files

Author : Lux
License : CC0

`LogBoop` was adapted from a bash script hastely written to handle
an unexpected surplus of log files rotated from my personal VPS.

### Using `LogBoop`
Using `LogBoop` is easy. Once the binary is compiled, simply invoke it
```bash
logboop input_root output_root
```

Note that you will need the required privilege to read all files and folders
in the `input_root` directory, create directories and files in
`output_root` (or create it as well if needed), and enough disk space to
duplicate the contents of `input_root` (roughly).

