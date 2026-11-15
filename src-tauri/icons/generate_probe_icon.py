"""Generate the original solid-color ICO used only by the feasibility harness."""
from pathlib import Path
import struct

size = 32
pixels = bytes((245, 180, 110, 255)) * size * size
mask = bytes(size * size // 8)
bitmap = struct.pack("<IiiHHIIiiII", 40, size, size * 2, 1, 32, 0,
                     len(pixels) + len(mask), 0, 0, 0, 0) + pixels + mask
header = struct.pack("<HHH", 0, 1, 1)
entry = struct.pack("<BBBBHHII", size, size, 0, 0, 1, 32, len(bitmap), 22)
Path(__file__).with_name("icon.ico").write_bytes(header + entry + bitmap)
