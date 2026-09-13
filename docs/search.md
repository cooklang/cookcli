# Search Command

Search through your recipe collection for matching text.

## Usage

```
cook search [OPTIONS] <TERMS>...
```

## Arguments

| Argument | Description |
|----------|-------------|
| `<TERMS>...` | One or more search terms. A recipe must match every one of them. |

## Options

| Option | Description |
|--------|-------------|
| `-b, --base-dir <DIR>` | Directory to search for recipes (default: current directory, recursive) |

## Examples

```bash
# Find recipes mentioning chicken
cook search chicken

# Find recipes mentioning both chicken and rice
cook search chicken rice

# Search in a specific directory
cook search -b ~/recipes pasta
```

## Notes

- Searches file names and the whole recipe text, including metadata
- Case-insensitive
- Every term must match, in the recipe text or in the file name, so extra
  terms narrow the results
- Results are ranked by relevance, best first: a file name matching the whole
  query outranks a recipe that merely mentions the terms
