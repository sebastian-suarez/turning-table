#!/usr/bin/env python3
"""Print binary STL bounds; reject malformed, open or non-manifold meshes.

No third-party packages. STL coordinates have no unit metadata: these exports use
millimetres. Exact shared float vertices are intentional here: welding nearby
vertices could conceal a slit in the mesh that will actually reach the slicer.
The topology checks do not detect geometric self-intersections; export with the
Manifold backend as well. See the Makefile for the default dimensional checks.
"""

import argparse
from collections import defaultdict
from dataclasses import dataclass
import math
from pathlib import Path
import struct
import sys


HEADER_SIZE = 84                    # 80 opaque bytes, then a little-endian uint32.
TRIANGLE = struct.Struct("<12fH")   # Normal, three vertices, two attribute bytes.
MAX_BYTES = 20_000_000              # Acceptance requires each file UNDER 20 MB.
DEFAULT_TOLERANCE = 0.05


@dataclass(frozen=True)
class Bounds:
    minimum: tuple
    maximum: tuple
    triangles: int
    byte_count: int

    @property
    def size(self):
        return tuple(hi - lo for lo, hi in zip(self.minimum, self.maximum))


def read_bounds(path):
    """Parse the binary records and check closed, consistently oriented topology."""
    byte_count = path.stat().st_size
    if byte_count < HEADER_SIZE:
        raise ValueError("short binary STL header (ASCII STL is not accepted)")
    if byte_count >= MAX_BYTES:
        raise ValueError(f"file must be under {MAX_BYTES} bytes")
    with path.open("rb") as stream:
        header = stream.read(HEADER_SIZE)
        triangle_count = struct.unpack_from("<I", header, 80)[0]
        expected = HEADER_SIZE + triangle_count * TRIANGLE.size
        if expected != byte_count:
            raise ValueError(f"binary length {byte_count} disagrees with header ({expected})")
        if not triangle_count:
            raise ValueError("empty mesh")
        records = stream.read()

    minimum = [math.inf] * 3
    maximum = [-math.inf] * 3
    edges = defaultdict(lambda: [0, 0])
    links = defaultdict(list)
    faces = set()
    for index, values in enumerate(TRIANGLE.iter_unpack(records), start=1):
        if not all(math.isfinite(value) for value in values[:12]):
            raise ValueError(f"triangle {index} has a non-finite coordinate or normal")
        vertices = (values[3:6], values[6:9], values[9:12])
        a, b, c = vertices
        ab = tuple(y - x for x, y in zip(a, b))
        ac = tuple(y - x for x, y in zip(a, c))
        cross = (ab[1] * ac[2] - ab[2] * ac[1],
                 ab[2] * ac[0] - ab[0] * ac[2],
                 ab[0] * ac[1] - ab[1] * ac[0])
        if not any(cross):
            raise ValueError(f"triangle {index} is degenerate")
        face = tuple(sorted(vertices))
        if face in faces:
            raise ValueError(f"triangle {index} duplicates a face")
        faces.add(face)
        for vertex in vertices:
            for axis, value in enumerate(vertex):
                minimum[axis] = min(minimum[axis], value)
                maximum[axis] = max(maximum[axis], value)
        # A closed oriented surface uses every edge twice, in opposite directions.
        for start, end in ((a, b), (b, c), (c, a)):
            key = tuple(sorted((start, end)))
            entry = edges[key]
            entry[0] += 1
            entry[1] += 1 if start < end else -1
        # Edge counts alone miss two closed shells touching at a single vertex.
        # At a manifold vertex the surrounding triangle link is one closed cycle.
        links[a].append((b, c))
        links[b].append((c, a))
        links[c].append((a, b))

    bad_edges = sum(count != 2 for count, _ in edges.values())
    if bad_edges:
        raise ValueError(f"{bad_edges} edges do not have exactly two incident triangles")
    bad_winding = sum(balance != 0 for _, balance in edges.values())
    if bad_winding:
        raise ValueError(f"{bad_winding} edges have inconsistent triangle winding")
    for vertex, pairs in links.items():
        neighbours = defaultdict(list)
        for start, end in pairs:
            neighbours[start].append(end)
            neighbours[end].append(start)
        if any(len(adjacent) != 2 for adjacent in neighbours.values()):
            raise ValueError(f"non-manifold vertex at {vertex}")
        visited = set()
        pending = [next(iter(neighbours))]
        while pending:
            current = pending.pop()
            if current not in visited:
                visited.add(current)
                pending.extend(neighbours[current])
        if len(visited) != len(neighbours):
            raise ValueError(f"disconnected triangle fans at vertex {vertex}")
    return Bounds(tuple(minimum), tuple(maximum), triangle_count, byte_count)


def triple(values):
    return "(" + ", ".join(f"{value:.3f}" for value in values) + ")"


def check_dimensions(bounds, args):
    for name, actual, expected in (("size", bounds.size, args.expect_size),
                                   ("minimum", bounds.minimum, args.expect_min)):
        if expected is not None and any(abs(a - e) > args.tolerance
                                        for a, e in zip(actual, expected)):
            raise ValueError(f"{name} {triple(actual)} differs from {triple(expected)} "
                             f"by more than {args.tolerance:g} mm")
    if args.max_size is not None and any(a > e + args.tolerance
                                         for a, e in zip(bounds.size, args.max_size)):
        raise ValueError(f"size {triple(bounds.size)} exceeds {triple(args.max_size)}")


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("paths", nargs="+", type=Path)
    parser.add_argument("--expect-size", nargs=3, type=float, metavar=("X", "Y", "Z"))
    parser.add_argument("--expect-min", nargs=3, type=float, metavar=("X", "Y", "Z"))
    parser.add_argument("--max-size", nargs=3, type=float, metavar=("X", "Y", "Z"))
    parser.add_argument("--tolerance", type=float, default=DEFAULT_TOLERANCE)
    args = parser.parse_args(argv)
    dimensions = [args.tolerance]
    for values in (args.expect_size, args.expect_min, args.max_size):
        if values is not None:
            dimensions.extend(values)
    if not all(math.isfinite(value) for value in dimensions) or args.tolerance < 0:
        parser.error("dimensions must be finite and tolerance must be nonnegative")
    failed = False
    for path in args.paths:
        try:
            bounds = read_bounds(path)
            size = " x ".join(f"{value:.3f}" for value in bounds.size)
            print(f"{path.name}: {size} mm; min={triple(bounds.minimum)}; "
                  f"max={triple(bounds.maximum)}; {bounds.triangles} triangles; "
                  f"closed manifold; {bounds.byte_count} bytes", flush=True)
            check_dimensions(bounds, args)
        except (OSError, ValueError, struct.error) as error:
            print(f"{path}: ERROR: {error}", file=sys.stderr)
            failed = True
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
