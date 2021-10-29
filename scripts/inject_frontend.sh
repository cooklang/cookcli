# run to inject static assets into code
# bash ./inject_frontend.sh

cd "$(dirname "$0")/../Sources/Server/Frontend" || exit

npm install
npm run build

cd ..

echo "Injecting html..."
INDEX_HTML=$(gzip -c ./Frontend/public/index.html | base64)
sed -i '' "s|let indexHTML.*|let indexHTML = \"$INDEX_HTML\"|g" ./Handler/StaticAssetsHandler.swift

echo "Injecting js..."
BUNDLE_JS=$(gzip -c ./Frontend/public/build/bundle.js | base64)
sed -i '' "s|let bundleJs.*|let bundleJs = \"$BUNDLE_JS\"|g" ./Handler/StaticAssetsHandler.swift

echo "Injecting css..."
BUNDLE_CSS=$(gzip -c ./Frontend/public/build/bundle.css | base64)
sed -i '' "s|let bundleCss.*|let bundleCss = \"$BUNDLE_CSS\"|g" ./Handler/StaticAssetsHandler.swift

echo "Injecting bootstrap css..."
BOOTSTRAP_CSS=$(gzip -c ./Frontend/public/vendor/bootstrap/css/bootstrap.min.css | base64)
sed -i '' "s|let bootstrapCss.*|let bootstrapCss = \"$BOOTSTRAP_CSS\"|g" ./Handler/StaticAssetsHandler.swift

echo "Injecting favicon..."
FAVICON=$(gzip -c ./Frontend/public/favicon.png | base64)
sed -i '' "s|let faviconPng.*|let faviconPng = \"$FAVICON\"|g" ./Handler/StaticAssetsHandler.swift

echo "Done"
