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
As of writing this document there are currently only two different run modes:
1. Directory mode (default) - will consume all `.go` files recursively for any given directories.
2. File mode - will consume all specified `.go` files. 

> NOTE : Files without a `.go` extension are ignored by the formatter.

To run the tool, use the following cmd line format:

```bash
#> ./goimpfmt <project local import> (-f) <directories / files to process>
```

### Directory Mode
The following is an example of running the tool in directory mode
```bash
#> ./goimpfmt github.com/Pungyeon/goimpfmt ~/projects/goimpfmt
```

### File Mode
The following is an example of running the tool in file mode
```bash
#> ./goimpfmt github.com/Pungyeon/goimpfmt -f ~/projects/goimpfmt/main.go
```

### Multiple input support
It's possible to add multiple directories. Simply append these at the end of the cmd, separating each with spaces:
```bash
#> ./goimpfmt github.com/Pungyeon/goimpfmt -f ~/projects/goimpfmt/main.go ~/projects/goimpfmt/main_test.go
```

> NOTE: This is also possible with the directory run mode.
