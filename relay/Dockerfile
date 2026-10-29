# The relay: one process, no state, no secret in the image. The
# platform assigns the port and the edge terminates TLS; the container
# only has to move frames it cannot read.
FROM python:3.12-slim

# An unprivileged user owns the process: the relay has nothing worth
# escalating for, and the image says so rather than implying it.
RUN useradd --create-home --shell /usr/sbin/nologin relay

WORKDIR /app
COPY relay.py .
USER relay

# The platform's PORT env is where the edge points; 8421 is the local
# default the reference has always used.
ENV PORT=8421
CMD ["python", "relay.py", "--host", "0.0.0.0"]
