# Server Command

The `server` command starts a local web server that makes your recipe collection browsable from any device with a web browser. It's perfect for cooking from a tablet, sharing recipes with family, or browsing your collection comfortably.

<img width="1166" height="995" alt="Screenshot 2025-08-28 at 16 47 49" src="https://github.com/user-attachments/assets/73ec0a6d-f2dc-4fcc-b54b-5622e0532df3" />
<img width="1175" height="935" alt="Screenshot 2025-08-28 at 16 47 56" src="https://github.com/user-attachments/assets/fdbfc722-cdec-401a-a9ac-6ff2bba4b7c5" />
<img width="1276" height="866" alt="Screenshot 2025-08-28 at 16 49 20" src="https://github.com/user-attachments/assets/8e6c0ffa-0957-4769-9268-beae8efdea7a" />


## Basic Usage

```bash
cook server
```

This starts a web server on `http://localhost:9080` serving recipes from the current directory.

## Starting the Server

### Serve Current Directory

```bash
cook server
# Server running at http://localhost:9080
```

### Serve Specific Directory

```bash
cook server ~/my-recipes
# Or
cd ~/my-recipes && cook server
```

### Custom Port

```bash
cook server --port 8080
# Server running at http://localhost:8080
```

### Allow External Access

By default, the server only accepts connections from localhost. To access from other devices:

```bash
cook server --host
# Server accessible at http://YOUR-IP:9080

# With custom port
cook server --host --port 3000
```

⚠️ **Security Note**: Only use `--host` on trusted networks. Your recipes will be accessible to anyone on the network.

### Auto-Open Browser

```bash
cook server --open
# Automatically opens http://localhost:9080 in your default browser
```

## Features

### Recipe Browsing

* **Tree view** of all recipes organized by folders
* **Search** across all recipes
* **Quick preview** with cooking time and servings
* **Full recipe view** with ingredients and steps

### Recipe Scaling

Scale any recipe directly in the web interface:
* Use the scaling controls to adjust servings
* All quantities update automatically
* Print or save the scaled version

### Shopping Lists

Create shopping lists from the web interface:
* Select multiple recipes
* Set scaling for each recipe
* Generate combined shopping list
* Export or print the list

### Mobile-Friendly

The web interface is responsive and works great on:
* Phones – Quick reference while shopping
* Tablets – Perfect for cooking
* Desktops – Comfortable browsing and planning

## Network Access

### Local Network

Share recipes with devices on your home network:

```bash
# Find your IP address
ip addr show  # Linux
ifconfig      # macOS

# Start server with external access
cook server --host --port 8080

# Access from other devices at:
# http://192.168.1.100:8080 (replace with your IP)
```

### Kitchen Setup

Ideal setup for cooking:

```bash
# On your computer
cook server ~/recipes --host

# On your tablet/phone
# Open browser to http://computer-ip:9080
```

### Family Sharing

Let family members browse recipes:

```bash
# Create a read-only recipe directory
cp -r ~/recipes ~/shared-recipes

# Serve the shared directory
cook server ~/shared-recipes --host --port 8080

# Family can access at http://your-ip:8080
```

## Advanced Usage

### Running as a Service

#### systemd (Linux)

Create `/etc/systemd/system/cooklang.service`:

```ini
[Unit]
Description=Cooklang Recipe Server
After=network.target

[Service]
Type=simple
User=YOUR-USER
WorkingDirectory=/home/YOUR-USER/recipes
ExecStart=/usr/local/bin/cook server --host --port 8080
Restart=always

[Install]
WantedBy=multi-user.target
```

Enable and start:

```bash
sudo systemctl enable cooklang
sudo systemctl start cooklang
```

#### launchd (macOS)

Create `~/Library/LaunchAgents/org.cooklang.server.plist`:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" 
  "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>org.cooklang.server</string>
    <key>ProgramArguments</key>
    <array>
        <string>/usr/local/bin/cook</string>
        <string>server</string>
        <string>/Users/YOUR-USER/recipes</string>
        <string>--port</string>
        <string>8080</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
</dict>
</plist>
```

Load the service:

```bash
launchctl load ~/Library/LaunchAgents/org.cooklang.server.plist
```

### Docker Deployment

Create a `Dockerfile`:

```dockerfile
FROM rust:latest
RUN cargo install cookcli
WORKDIR /recipes
COPY ./recipes /recipes
EXPOSE 9080
CMD ["cook", "server", "--host"]
```

Build and run:

```bash
docker build -t my-recipes .
docker run -p 9080:9080 my-recipes
```

### Reverse Proxy

Use with nginx for production deployment:

```nginx
server {
    listen 80;
    server_name recipes.example.com;

    location / {
        proxy_pass http://localhost:9080;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }
}
```

## Web Interface Guide

### Home Page

* **Recipe tree** – Browse by folder structure
* **Search bar** – Find recipes quickly
* **Recent recipes** – Quick access to recently viewed

### Recipe View

* **Ingredients panel** – Checklist with quantities
* **Steps** – Clear cooking instructions
* **Scaling control** – Adjust servings dynamically
* **Timer highlights** – Click to start timers
* **Print view** – Optimized for printing

### Shopping List Builder

1. Click "Shopping List" in navigation
2. Select recipes to include
3. Set quantities for each
4. Click "Generate List"
5. Print or export the results

## Tips and Tricks

### Quick Access

Create shortcuts for common uses:

```bash
# Add to ~/.bashrc or ~/.zshrc
alias recipes='cook server ~/recipes --open'
alias cookbook='cook server ~/cookbook --host --port 8080'
```

### Bookmarks

Save commonly used recipe URLs:
* `http://localhost:9080/recipes/favorites/`
* `http://localhost:9080/recipe/Pizza.cook`
* `http://localhost:9080/shopping-list`

### Tablet Mode

For dedicated kitchen tablet:

1. Start server with `--host`
2. Create tablet bookmark to server URL
3. Use "Add to Home Screen" for app-like experience
4. Enable "Reader Mode" for cleaner display

### Development Workflow

When developing recipes:

```bash
# Auto-reload on file changes (using external tools)
fswatch -o ~/recipes | xargs -n1 -I{} curl http://localhost:9080/api/reload

# Split terminal: edit and preview
# Terminal 1: Editor
vim ~/recipes/new-recipe.cook
# Terminal 2: Server
cook server ~/recipes --open
```

## Troubleshooting

### Port Already in Use

```bash
# Check what's using the port
lsof -i :9080  # macOS/Linux

# Use a different port
cook server --port 8081
```

### Can't Access from Other Devices

1. Check firewall settings
2. Ensure using `--host` flag
3. Verify IP address is correct
4. Check network connectivity

### Slow Loading

* Large recipe collections may take time to index
* Consider organizing recipes into folders
* Reduce image sizes in recipe directories

## Security Considerations

### Local Network Only

By default, the server only accepts local connections. This is the safest option.

### Network Access

When using `--host`:
* Only use on trusted networks
* Consider using a firewall
* Don't expose to the internet directly
* Use reverse proxy with authentication for public access

### Read-Only Access

The web interface provides read-only access to recipes. Files cannot be modified through the web interface.

## See Also

* [Recipe](recipe.md) – Command-line recipe viewing
* [Shopping List](shopping-list.md) – Generate shopping lists via CLI
* [Search](search.md) – Search recipes from command line
