echo "Injecting html..."
cd ./seed
SEED=$(zip -r - ./ | base64)
sed -i '' "s|let seed.*|let seed = \"$SEED\"|g" ../Sources/CookCLI/Commands/Seed.swift
