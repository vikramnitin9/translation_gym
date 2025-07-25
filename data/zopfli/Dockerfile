FROM ubuntu:20.04

RUN apt -y update
RUN apt -y upgrade
RUN apt install -y build-essential
RUN apt install -y python3

ENV DEBIAN_FRONTEND=noninteractive

WORKDIR /app

COPY ./zopfli/test.py /app/test.py
RUN mkdir -p /executable
ENV PATH="/executable:${PATH}"

ENV INSTRUMENTATION_PATH="/instrumentation"

CMD ["/bin/bash", "-c", "python3 /app/test.py"]