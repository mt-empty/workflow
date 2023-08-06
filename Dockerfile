FROM debian:12

RUN apt-get update && apt-get install -y libpq-dev iputils-ping postgresql-client # needed to setup migrations

WORKDIR /usr/src/workflow
COPY --chown=1000:1000 /target/release/workflow .
RUN chown -R 1000:1000 /usr/src/workflow
# USER 1000
CMD ["/bin/sh", "-c", "./workflow start && tail -f /dev/null"]