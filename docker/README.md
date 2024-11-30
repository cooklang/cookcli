To build the latest version of Cooklang webserver:      

```
docker build -t cookcli .
```

To run the latest version of Cooklang webserver, first create a .env file and set the ROOT variable to the directory where you want to store your recipes. Then run the following command:     

```
docker compose up -d
```

The webserver will be available at http://localhost:9080 on the host machine, or at the IP address of the host machine if you are running it on a remote machine.

To stop the webserver, run the following command:     

```
docker compose down
```
