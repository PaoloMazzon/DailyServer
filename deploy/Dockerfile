FROM debian:stable-slim
ARG DEBIAN_FRONTEND=noninteractive
COPY daily_server.deb /tmp
RUN apt-get install -y /tmp/daily_server.deb \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /opt/daily_server
ENTRYPOINT ["daily_server"]
