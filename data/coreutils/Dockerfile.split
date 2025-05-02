FROM coreutils:latest

RUN ["rm", "-f", "/app/src/split"]
RUN ["sudo", "rm", "-f", "/usr/bin/split"]

# Failing - /app/tests/split/r-chunk.sh

CMD ["bash", "-c", "make TESTS=\"tests/split/line-bytes.sh \
     tests/split/b-chunk.sh \
     tests/split/l-chunk.sh \
     tests/split/suffix-length.sh \
     tests/split/numeric.sh \
     tests/split/suffix-auto-length.sh \
     tests/split/guard-input.sh \
     tests/split/fail.sh \
     tests/split/filter.sh \
     tests/split/additional-suffix.sh \
     tests/split/lines.sh \
     tests/split/record-sep.sh \" \
     SUBDIRS=. VERBOSE=yes check && \
     sudo make TESTS=\"tests/split/l-chunk-root.sh\" SUBDIRS=. VERBOSE=yes check && \
    { [ -f \"/app/instrumentation/instrumented.json\" ] \
      && sudo cp /app/instrumentation/instrumented.json \"/instrumentation/\"; true; }"]