# CookCLI

CookCLI is provided as a command-line tool to make Cook recipe management easier, and enable automation and scripting workflows for the CookLang ecosystem.

## Installation

Download latest release for your platform from the [releases page](https://github.com/cooklang/CookCLI/releases) and add the file to your operating system's PATH.

On Linux (or [WSL](https://docs.microsoft.com/en-us/windows/wsl/about)), this is easy. Simply extract the binary into your binaries folder with `sudo unzip CookCLI_1.0.0_linux_amd64.zip -d /usr/local/bin/` (note: you may need to install the unzip package first). 

TODO: Mac install.
    
## Usage
`cook` is a toolkit for command-line interaction with CookLang text files
You can find full documentation at https://cooklang.org/cli/help/ and report issues at https://github.com/CookLang/CookCLI

```
Usage: cook [OPTIONS] COMMAND

Options:
    -a, --aisle <aisle>             Specify an aisle.conf file to override shopping list default settings 
    -u, --units <units>             Specify a units.conf file to override units default settings
    -i, --inflection <inflection>   Specify an inflection.conf file to override default inflection settings
    -h, --help                      Show help information.

Commands:
    recipe                  Manage recipes and recipe files
    shopping-list           Create a shopping list
    server                  Run a webserver to serve your recipes on the web
    fetch                   Pull recipes from the community recipe repository
    version                 Show the CookCLI version information
    help                    Show the help text for a sub-command
```

## Contribution

Please open issues for any ideas you may have to contribute to the project.

## License

MIT License

Copyright (c) 2021 Alexey Dubovskoy

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
