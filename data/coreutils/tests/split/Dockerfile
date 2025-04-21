FROM coreutils:latest

RUN ["rm", "-f", "/app/src/split"]
RUN ["sudo", "rm", "-f", "/user/bin/split"]

# Failing - /app/tests/split/r-chunk.sh

CMD ["bash", "-c", "make TESTS=\"/app/tests/split/line-bytes.sh \
     /app/tests/split/b-chunk.sh \
     /app/tests/split/l-chunk.sh \
     /app/tests/split/suffix-length.sh \
     /app/tests/split/numeric.sh \
     /app/tests/split/suffix-auto-length.sh \
     /app/tests/split/guard-input.sh \
     /app/tests/split/fail.sh \
     /app/tests/split/filter.sh \
     /app/tests/split/additional-suffix.sh \
     /app/tests/split/lines.sh \
     /app/tests/split/record-sep.sh \" \
     SUBDIRS=. VERBOSE=yes check && \
     sudo make TESTS=\"/app/tests/split/l-chunk-root.sh\" SUBDIRS=. VERBOSE=yes check && \
    { [ -f \"/app/instrumentation/instrumented.json\" ] \
      && sudo cp /app/instrumentation/instrumented.json \"/instrumentation/\"; true; }"]