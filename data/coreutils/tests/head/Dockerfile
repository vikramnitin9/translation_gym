FROM coreutils:latest

RUN ["rm", "-f", "/app/src/head"]
RUN ["sudo", "rm", "-f", "/user/bin/head"]

# Failing - tests/head/head-c.sh

CMD ["bash", "-c", "make TESTS=\"tests/head/head-pos.sh \
                                tests/head/head-write-error.sh \
                                tests/head/head-elide-tail.pl \
                                tests/head/head.pl \" \
                            SUBDIRS=. VERBOSE=yes check && \
                            { [ -f \"/app/instrumentation/instrumented.json\" ] \
                            && sudo cp /app/instrumentation/instrumented.json \"/instrumentation/\"; true; }"]