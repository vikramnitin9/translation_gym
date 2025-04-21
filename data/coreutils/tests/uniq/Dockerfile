FROM coreutils:latest

RUN ["rm", "-f", "/app/src/uniq"]
RUN ["sudo", "rm", "-f", "/user/bin/uniq"]

CMD ["bash", "-c", "make TESTS=\"tests/uniq/uniq-collate.sh \
                            tests/uniq/uniq-perf.sh \
                            tests/uniq/uniq.pl\" \
    SUBDIRS=. VERBOSE=yes check && \
    { [ -f \"/app/instrumentation/instrumented.json\" ] \
      && sudo cp /app/instrumentation/instrumented.json \"/instrumentation/\"; true; }"]
      