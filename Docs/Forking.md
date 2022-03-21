# Forking CookCLI

Community members wishing to contribute code to `CookCLI` must fork the `CookCLI` project
(`your-github-username/CookCLI`). Branches pushed to that fork can then be submitted
as pull requests to the upstream project (`cooklang/CookCLI`).

To locally clone the repo so that you can pull the latest from the upstream project
(`cooklang/CookCLI`) and push changes to your own fork (`your-github-username/CookCLI`):

1. [Create the forked repository](https://docs.github.com/en/get-started/quickstart/fork-a-repo#forking-a-repository) (`your-github-username/CookCLI`)
2. Clone the `cooklang/CookCLI` repository and `cd` into the folder
3. Make `cooklang/CookCLI` the `upstream` remote rather than `origin`:
   `git remote rename origin upstream`.
4. Add your fork as the `origin` remote. For example:
   `git remote add origin https://github.com/myusername/CookCLI`
5. Checkout a feature branch: `git checkout -t -b new-feature`
6. [Make changes](../CONTRIBUTING.md#prerequisites).
7. Push changes to the fork when ready to [submit a PR](../CONTRIBUTING.md#submitting-a-pull-request):
   `git push -u origin new-feature`
