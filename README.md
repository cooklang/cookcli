# CookCLI

The CookCLI tool is designed to make command line and shell scripting automation possible for the CookLang ecosystem.

## Installation

Download latest release for your platform from [Releases](https://github.com/cooklang/CookCLI/releases) and add file to executable path.

For example:

    tar xvfz CookCLI_0.0.1_darwin_amd64.tar.gz; rm CookCLI_0.0.1_darwin_amd64.tar.gz; \
    mv cook /usr/local/bin/;
    
## Usage

### Getting help

The below commands are all way of getting help.

    cook
    cook -h
    cook --help

    #EXAMPLE OUTPUT
    OVERVIEW: A toolkit for command-line interaction with CookLang text files.
    Documentation can be found at https://cooklang.org/cli/help/ and issues reported at https://github.com/CookLang/CookCLI.

    USAGE: cook [--aisle <aisle>] [--units <units>] [--inflection <inflection>] <subcommand>

    OPTIONS:
      -a, --aisle <aisle>     Specify an aisle.conf file to override shopping list default settings (TODO) 
      -u, --units <units>     Specify a units.conf file to override units default settings (TODO) 
      -i, --inflection <inflection>
                              Specify an inflection.conf file to override default inflection settings (TODO) 
      -h, --help              Show help information.

    SUBCOMMANDS:
      recipe                  Manage recipes and recipe files
      shopping-list           Create a shopping list
      server                  Run a webserver to serve your recipes on the web
      fetch                   Pull recipes from the community recipe repository
      version                 Show the CookCLI version information (TODO)

      See 'cook help <subcommand>' for detailed help.
      
## Contribution

Feel free to share ideas and criticism or open pull requests.

## License

TBD
