FROM ubuntu:22.04

RUN mkdir -p /app

WORKDIR /app

ADD dist/compute_data_runner /compute_data_runner
