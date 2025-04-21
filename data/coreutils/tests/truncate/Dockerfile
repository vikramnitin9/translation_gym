FROM coreutils:latest

RUN ["rm", "-f", "/app/src/truncate"]
RUN ["sudo", "rm", "-f", "/user/bin/truncate"]

CMD ["bash", "-c", "make TESTS=\"/app/tests/truncate/truncate-no-create-missing.sh \
      /app/tests/truncate/truncate-relative.sh \
      /app/tests/truncate/truncate-fail-diag.sh \
      /app/tests/truncate/truncate-dangling-symlink.sh \
      /app/tests/truncate/truncate-fifo.sh \
      /app/tests/truncate/truncate-parameters.sh \
      /app/tests/truncate/truncate-dir-fail.sh \
      /app/tests/truncate/truncate-overflow.sh\" \
    SUBDIRS=. VERBOSE=yes check && \
    sudo make TESTS=\"/app/tests/truncate/truncate-owned-by-other.sh\" \
    SUBDIRS=. VERBOSE=yes check && \
    { [ -f \"/app/instrumentation/instrumented.json\" ] \
      && sudo cp /app/instrumentation/instrumented.json \"/instrumentation/\"; true; }"]
      

