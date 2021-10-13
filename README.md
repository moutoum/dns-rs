# dns-rs
Self learning project which tries to implement DNS protocol and DNS servers.

## Example


```
./resolver -b 0.0.0.0:5465
Starting server on 0.0.0.0:5465
Looking up of A for www.google.com with 198.41.0.4
Looking up of A for www.google.com with 192.12.94.30
Looking up of A for www.google.com with 216.239.34.10
```

```
dig www.google.com @localhost -p 5465

; <<>> DiG 9.10.6 <<>> www.google.com @localhost -p 5465
;; global options: +cmd
;; Got answer:
;; ->>HEADER<<- opcode: QUERY, status: NOERROR, id: 20250
;; flags: qr aa rd ra; QUERY: 1, ANSWER: 1, AUTHORITY: 0, ADDITIONAL: 0

;; QUESTION SECTION:
;www.google.com.                        IN      A

;; ANSWER SECTION:
www.google.com.         300     IN      A       142.250.75.228

;; Query time: 76 msec
;; SERVER: 127.0.0.1#5465(127.0.0.1)
;; WHEN: Wed Oct 13 14:36:17 CEST 2021
;; MSG SIZE  rcvd: 62

```
