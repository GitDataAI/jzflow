FROM ubuntu:22.04

RUN apt-get update -y && \
    apt-get install -y bash-completion command-not-found mtr-tiny dnsutils net-tools && \
    apt-get install -y nmap traceroute netcat iproute2 tcpdump iputils-ping isc-dhcp-client && \
    apt-get install -y openssh-server openssh-client tmux screen vim nano && \
    apt-get install -y telnet && \
    apt-get install -y socat && \
    apt-get clean -qy
