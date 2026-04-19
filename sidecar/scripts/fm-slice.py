#!/usr/bin/env python3
"""
Slice a Forza Dash pcapng down to a fixture-sized file:
  - keep only UDP packets to the configured dst port (default 5300)
  - keep only the byte range corresponding to forza-stream indices [first_idx, last_idx]
  - preserve original timestamps + the original SHB/IDB so Wireshark/scapy still parse it

Usage: fm-slice.py <input.pcapng> <output.pcapng> <first_idx> <last_idx> [dport=5300]

Indices are into the dst-port-5300 stream (the same indices fm-analyze.py emits),
NOT into the raw capture.
"""

import struct
import sys
from pathlib import Path

PCAPNG_BLOCK_SHB = 0x0A0D0D0A
PCAPNG_BLOCK_IDB = 0x01
PCAPNG_BLOCK_EPB = 0x06


def slice_pcapng(src_path, dst_path, first_forza_idx, last_forza_idx, dport=5300):
    src = open(src_path, "rb")
    dst = open(dst_path, "wb")

    forza_idx = -1
    kept = 0
    while True:
        hdr = src.read(8)
        if len(hdr) < 8:
            break
        bt, blen = struct.unpack("<II", hdr)
        body = src.read(blen - 12)
        trailer = src.read(4)

        if bt in (PCAPNG_BLOCK_SHB, PCAPNG_BLOCK_IDB):
            # always copy section / interface blocks verbatim
            dst.write(hdr + body + trailer)
        elif bt == PCAPNG_BLOCK_EPB:
            if len(body) < 20:
                continue
            _iface, _th, _tl, cap_len, _orig = struct.unpack("<IIIII", body[:20])
            frame = body[20 : 20 + cap_len]
            # NULL/loopback: 4-byte family + IPv4
            if len(frame) < 4 + 20 + 8:
                continue
            ip = frame[4:]
            if ip[9] != 17:
                continue
            ihl = (ip[0] & 0x0F) * 4
            udp = ip[ihl:]
            if struct.unpack("!H", udp[2:4])[0] != dport:
                continue
            payload = udp[8:]
            if len(payload) != 331:
                continue
            forza_idx += 1
            if forza_idx < first_forza_idx:
                continue
            if forza_idx > last_forza_idx:
                break
            dst.write(hdr + body + trailer)
            kept += 1

    src.close()
    dst.close()
    return kept


def main():
    if len(sys.argv) < 5:
        print(__doc__)
        sys.exit(1)
    src = sys.argv[1]
    dst = sys.argv[2]
    first = int(sys.argv[3])
    last = int(sys.argv[4])
    dport = int(sys.argv[5]) if len(sys.argv) > 5 else 5300
    kept = slice_pcapng(src, dst, first, last, dport)
    sz = Path(dst).stat().st_size
    print(f"wrote {dst}: {kept} packets, {sz/1e6:.2f} MB")


if __name__ == "__main__":
    main()
