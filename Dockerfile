FROM docker.io/alpine:latest

#Set download URL (customize if not using amd64 CPU)
ARG DOWNLOAD_URL="https://github.com/cooklang/cookcli/releases/latest/download/cook-x86_64-unknown-linux-musl.tar.gz"

RUN apk upgrade --no-cache

#Add data dir (mount volume with your recipes and config dir here)
RUN mkdir /data --mode=555
WORKDIR /data

#Add cookcli binary
ADD ${DOWNLOAD_URL} .

#Untar binary
RUN tar -xvf cook-*

#Remove tar
RUN rm *.tar.gz

#Install binary
RUN mv cook /bin/cook && chmod 555 /bin/cook && chown root /bin/cook && chgrp root /bin/cook

#Add non-root user (optional)         
RUN addgroup -g 1000 cookcli_user
RUN adduser -u 1000 -G cookcli_user -s /bin/sh -D cookcli_user
RUN chown -R 1000 /data && chgrp -R 1000 /data
USER cookcli_user

#Run server
EXPOSE 9080
ENTRYPOINT cook server --host /data
