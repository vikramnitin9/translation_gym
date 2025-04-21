FROM coreutils:latest

RUN ["rm", "-f", "/app/src/uniq"]
RUN ["sudo", "rm", "-f", "/user/bin/uniq"]

CMD ["bash", "-c", "make TESTS=\"/app/tests/uniq/uniq-collate.sh \
                            /app/tests/uniq/uniq-perf.sh \
                            /app/tests/uniq/uniq.pl\" \
    SUBDIRS=. VERBOSE=yes check && \
    { [ -f \"/app/instrumentation/instrumented.json\" ] \
      && sudo cp /app/instrumentation/instrumented.json \"/instrumentation/\"; true; }"]
      