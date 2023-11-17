# CookCLI

CookCLI provides a suite of tools to create shopping lists and maintain recipes. We've built it to be simple and useful for automating your cooking and shopping routine with existing UNIX command line and scripting tools. It can also function as a webserver for your recipes, making them browsable on any device with a web browser.

* [Example usage](#example-usage)  
* [Installation](#installation)  
* [Building from source](#building-from-source)  
* [Contribution](#contribution)  
* [License](#license)  

## Example usage

Add sample recipes:

```
$ cook seed
$ tree .
.
├── Baked Potato Soup.cook
...
├── Neapolitan Pizza.cook
...
├── README.md
├── Root Vegetable Tray Bake.cook
...
└── config
    └── aisle.conf

3 directories, 15 files
```

Check "Neapolitan Pizza":
```
$ cook recipe read "Neapolitan Pizza.cook"
Metadata:
    servings: 6

Ingredients:
    chopped tomato                3 cans
    dried oregano                 3 tbsp
    fresh basil                   18 leaves
    fresh yeast                   1.6 g
    garlic                        3 gloves
    mozzarella                    3 packs
    parma ham                     3 packs
    salt                          25 g
    tipo zero flour               820 g
    water                         530 ml

Steps:
     1. Make 6 pizza balls using tipo zero flour, water, salt and fresh yeast. Put in a fridge for 2 days.
        [fresh yeast: 1.6 g; salt: 25 g; tipo zero flour: 820 g; water: 530 ml]
     2. Set oven to max temperature and heat pizza stone for about 40 minutes.
        [–]
     3. Make some tomato sauce with chopped tomato and garlic and dried oregano. Put on a pan and leave for 15 minutes occasionally stirring.
        [chopped tomato: 3 cans; dried oregano: 3 tbsp; garlic: 3 gloves]
     4. Make pizzas putting some tomato sauce with spoon on top of flattened dough. Add fresh basil, parma ham and mozzarella.
        [fresh basil: 18 leaves; mozzarella: 3 packs; parma ham: 3 packs]
     5. Put in an oven for 4 minutes.
        [–]

```

Create a shopping list:
```
$ cook shopping-list \
> Neapolitan\ Pizza.cook \
> Root\ Vegetable\ Tray\ Bake.cook
BREADS AND BAKED GOODS
    breadcrumbs                   150 g

DRIED HERBS AND SPICES
    dried oregano                 3 tbsp
    dried sage                    1 tsp
    pepper                        1 pinch
    salt                          25 g, 2 pinches

FRUIT AND VEG
    beetroots                     300 g
    carrots                       300 g
    celeriac                      300 g
    fresh basil                   18 leaves
    garlic                        3 gloves
    lemon                         1 item
    onion                         1 large
    red onion                     2 items
    thyme                         2 springs

MEAT AND SEAFOOD
    parma ham                     3 packs

MILK AND DAIRY
    butter                        15 g
    egg                           1 item
    mozzarella                    3 packs

OILS AND DRESSINGS
    Dijon mustard                 1 tsp
    Marmite                       1 tsp
    cider                         150 ml
    olive oil                     3 tbsp

OTHER (add new items into aisle.conf)
    tipo zero flour               820 g

PACKAGED GOODS, PASTA AND SAUCES
    vegetable stock               150 ml
    water                         530 ml

TINNED GOODS AND BAKING
    cannellini beans              400 g
    chopped tomato                3 cans
    fresh yeast                   1.6 g
    redcurrant jelly              1 tsp
```

Run a web-server:

    $ cook server
    Started server on http://127.0.0.1:9080, serving cook files from /Users/pochho/recipes.

![server](https://user-images.githubusercontent.com/4168619/148116974-7010e265-5aa8-4990-a4b9-f85abe3eafb0.png)


You can find full documentation at https://cooklang.org/cli/help/ or by running `help` command.

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

## Installation

Download latest release for your platform from the [releases page](https://github.com/cooklang/CookCLI/releases) and add the file to your operating system's PATH.

On Linux (or [WSL](https://docs.microsoft.com/en-us/windows/wsl/about)), this is easy. Simply extract the binary into your binaries folder with `sudo unzip CookCLI_1.0.0_linux_amd64.zip -d /usr/local/bin/` (note: you may need to install the unzip package first). 

On MacOS:

    brew tap cooklang/tap
    brew install cooklang/tap/cook
    
## Building from source

1. Checkout code.
2. Install Swift by following official [instructions](https://swift.org/getting-started/#installing-swift).
3. Build CookCLI from a directory with the source code:

```
swift build --configuration release -static-executable
```
4. Take binary from `.build/x86_64-unknown-linux-gnu/release/cook` or `.build/x86_64-apple-macosx/release/cook`

Note. If you don't want to install Swift, there's a `Dockerfile` for building Linux binary:

    docker build -t cook-builder .
    docker run  --volume $(pwd):/src --workdir /src --entrypoint "swift" -it cook-builder build --configuration release -Xswiftc -static-executable


## Contribution

Please open issues for any ideas you may have to contribute to the project.

## License

MIT License

Copyright (c) 2021-2022 Alexey Dubovskoy

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

Some files include a substantial portion of code from
https://github.com/Zheoni/cooklang-chef.

The original code is licensed under the MIT License, a copy of which
is provided in these files in addition to our project's license.
