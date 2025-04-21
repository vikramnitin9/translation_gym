FROM coreutils:latest

RUN ["rm", "-f", "/app/src/cat"]
RUN ["sudo", "rm", "-f", "/user/bin/cat"]

CMD ["bash", "-c", "make TESTS=\"tests/cat/cat-E.sh \
                                tests/cat/cat-self.sh \
                                tests/cat/cat-proc.sh \
                                tests/cat/cat-buf.sh\" \
                    SUBDIRS=. VERBOSE=yes check && \
                   { [ -f \"/app/instrumentation/instrumented.json\" ] \
                     && sudo cp /app/instrumentation/instrumented.json \"/instrumentation/\"; true; }"]