echo "Injecting seed..."
cd ./seed
SEED=$(zip -r - ./ | base64)
echo "s|let seed.*|let seed = \"$SEED\"|g" > inject_seed_command
sed -i '' -f inject_seed_command ../Sources/CookCLI/Commands/Seed.swift
