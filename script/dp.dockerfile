FROM ubuntu:22.04

RUN mkdir -p /app

WORKDIR /app

ADD dist/dp_runner /app/dp_runner

CMD ["compute_data_runner"]