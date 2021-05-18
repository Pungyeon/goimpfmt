# Go Import Formatter
## Overview
The purpose of this tool is rather simple: To format Go file imports in a specific manner.

The tool will ensure that formatted files adhere to the following import structure:

```
import (
    <built-in libraries>
    
    <local package libraries>
    
    <external libraries>
)
```

## Using the cmd line tool
To run the tool, use the following cmd line format (commands in parenthesis are optional):

```bash
#> ./goimpfmt (-w) (-q) --project <project local import> --input <directories / files to process>
```

Use the `-h` flag to output the following information:
```bash 
Go Import Format 0.3.0
Lasse Martin Jakobsen (Pungyeon)
Formats Go imports enforcing the Vivino style guide, grouping and separating built-in, internal and
external library imports

USAGE:
    import-fix [FLAGS] --project <PROJECT IMPORT> --input <FILE/DIRECTORY>...

FLAGS:
    -h, --help       Prints help information
    -q, --quiet      will suppress diff output
    -V, --version    Prints version information
    -w, --write      specifies whether to write any eventual diff to the formatted file /
                     directories

OPTIONS:
    -i, --input <FILE/DIRECTORY>...    specifies the directories and/or files to format. Directories
                                       are formatted recursively.
    -p, --project <PROJECT IMPORT>     determines the 'internal' import prefix

```

> NOTE : Files without a `.go` extension are ignored by the formatter.

### Multiple input support
It's possible to add multiple directories and/or files. Simply append these at the end of the cmd, separating each with spaces:
```bash
#> ./goimpfmt github.com/Pungyeon/goimpfmt -f ~/projects/goimpfmt/lib ~/projects/goimpfmt/main_test.go
```

> NOTE: This is also possible with the directory run mode.

