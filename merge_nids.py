import csv
import sys
import os

NIDS_CSV = r"C:\Users\claimoar\Documents\Rust\ps5rs\data\nids.csv"
NEW_TXT = r"C:\Users\claimoar\Documents\Rust\New PS5 Nids.txt"

B64_CHARS = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+-"

def nid_to_u64(nid: str) -> int | None:
    if len(nid) != 11:
        return None
    values = []
    for c in nid:
        idx = B64_CHARS.find(c)
        if idx == -1:
            return None
        values.append(idx)
    bytes_arr = bytearray(8)
    # group 0: chars 0-3 -> bytes 0-2
    bytes_arr[0] = ((values[0] << 2) | (values[1] >> 4)) & 0xFF
    bytes_arr[1] = (((values[1] & 0x0F) << 4) | (values[2] >> 2)) & 0xFF
    bytes_arr[2] = (((values[2] & 0x03) << 6) | values[3]) & 0xFF
    # group 1: chars 4-7 -> bytes 3-5
    bytes_arr[3] = ((values[4] << 2) | (values[5] >> 4)) & 0xFF
    bytes_arr[4] = (((values[5] & 0x0F) << 4) | (values[6] >> 2)) & 0xFF
    bytes_arr[5] = (((values[6] & 0x03) << 6) | values[7]) & 0xFF
    # chars 8-10 -> bytes 6-7
    bytes_arr[6] = ((values[8] << 2) | (values[9] >> 4)) & 0xFF
    bytes_arr[7] = (((values[9] & 0x0F) << 4) | (values[10] >> 2)) & 0xFF
    return int.from_bytes(bytes_arr, byteorder='big')

def main():
    # Load existing NIDs
    existing = set()
    if os.path.exists(NIDS_CSV):
        with open(NIDS_CSV, newline='', encoding='utf-8') as f:
            reader = csv.reader(f)
            header = next(reader, None)
            for row in reader:
                if row:
                    existing.add(row[0])
    new_rows = []
    with open(NEW_TXT, encoding='utf-8') as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            parts = line.split(None, 1)
            if len(parts) < 2:
                continue
            nid, name = parts[0], parts[1]
            if nid in existing:
                continue
            value = nid_to_u64(nid)
            if value is None:
                print(f"skipping invalid nid: {nid}", file=sys.stderr)
                continue
            nid_hex = f"0x{value:016X}"
            new_rows.append([nid, nid_hex, name, "", "0", "New PS5 Nids.txt"])
            existing.add(nid)
    if not new_rows:
        print("No new NIDs to add.")
        return
    with open(NIDS_CSV, 'a', newline='', encoding='utf-8') as f:
        writer = csv.writer(f)
        writer.writerows(new_rows)
    print(f"Added {len(new_rows)} new NIDs to {NIDS_CSV}")

if __name__ == "__main__":
    main()
