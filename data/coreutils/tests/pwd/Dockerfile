FROM coreutils:latest

RUN ["rm", "-f", "/app/src/pwd"]
RUN ["sudo", "rm", "-f", "/user/bin/pwd"]

CMD ["bash", "-c", "make TESTS=\"tests/pwd/pwd-long.sh \
                                tests/pwd/pwd-option.sh\" \
                    SUBDIRS=. VERBOSE=yes check && \
                   { [ -f \"/app/instrumentation/instrumented.json\" ] \
                     && sudo cp /app/instrumentation/instrumented.json \"/instrumentation/\"; true; }"]