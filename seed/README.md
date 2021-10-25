# What's next

* Install a syntax highlighting package for your text editor. We have packages for SublimeText and VSCode. See  [cooklang.org](https://cooklang.org/docs/syntax-highlighting/) for full instructions.
* Add your own recipes. Dive into the Cook ecoysystem and discover how easy it is to write in CookLang. It's the best way to learn the [CookLang syntax](https://cooklang.org/docs/spec/).
* Check out our [tips and tricks](https://cooklang.org/docs/best-practices/) page.

### Read the recipe

```sh
cook recipe read "Root Vegetable Tray Bake.cook"
```

### Create shopping list

```sh
cook shopping-list \
  "Neapolitan Pizza.cook" \
  "Root Vegetable Tray Bake.cook" \
  "Snack Basket I.cook"
```

### Run a server

In directory where you have your recipes run:

```sh
cook server
```

Then open [http://127.0.0.1:9080](http://127.0.0.1:9080) in your browser.

### Automate something

Explore [the docs](https://cooklang.org/cli/help/), which describe how to use CookCLI's automation tools.

### Customize your instance

Add aisle configuration information to the `config/aisle.conf` file to tailor your shopping list experience.




