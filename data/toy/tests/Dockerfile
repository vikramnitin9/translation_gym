FROM ubuntu:20.04

RUN apt -y update
RUN apt -y upgrade
RUN apt install -y build-essential

ENV DEBIAN_FRONTEND=noninteractive

RUN apt install -y cmake
RUN apt install -y wget
RUN apt install -y unzip
RUN apt install -y curl

WORKDIR /app

COPY toy/tests/test.sh /app/test.sh
RUN mkdir -p /executable
ENV PATH="/executable:${PATH}"
# Make test.sh executable
RUN chmod +x /app/test.sh

CMD ["/bin/bash", "/app/test.sh"]