# What's next

* Install syntax highlighting. We support SublimeText and VSCode, please check how to install them on [the web-site](https://cooklang.org/docs/syntax-highlighting/).
* Add your recipe. It's the best way to learn [CookLang syntax](https://cooklang.org/docs/spec/).
* Learn some [tips and tricks](https://cooklang.org/docs/best-practices/)

### Read the recipe

```
cook recipe read Root\ Vegetable\ Tray\ Bake.cook
```

### Create shopping list

```
cook shopping-list \
  "Neapolitan Pizza.cook" \
  "Root Vegetable Tray Bake.cook" \
  "Snack Basket I.cook"
```

### Run server

In directory where you have your recipes run:

```
cook server
```

Then open [http://127.0.0.1:9080](http://127.0.0.1:9080) in your browser.

### Use in scripting

Explore some optioins in [the doc](https://cooklang.org/cli/help/) which allow to use json format and set more machine friendly output.

### Make your configuation file

Add ingredients from your recipe which weren't grouped by aisle into `config/aisle.conf`.




