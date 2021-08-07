FROM swift

RUN apt-get update && apt-get install -y \
    libcurl4-openssl-dev \
    libxml2-dev && \
    rm -r /var/lib/apt/lists/*

