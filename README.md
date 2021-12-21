# Bagextract

Beating the database with a CPU. Currently does in 0.4 seconds what takes the DB over 3 minutes:

```
[src/main.rs:105] pretty = [
    "1181GG",
    "1181GX",
    "1181GZ",
    "1181TR",
    "1183PR",
    "1183PS",
    "3526LB",
    "3768CC",
    "4587LG",
    "4824CS",
    "4824CT",
    "4838EK",
    "6535SH",
    "6535SK",
    "6535VL",
    "6535WT",
]

________________________________________________________
Executed in  411.53 millis    fish           external
   usr time  379.29 millis  495.00 micros  378.79 millis
   sys time   32.06 millis  165.00 micros   31.90 millis
```

## General idea

We want to find postcodes that are close to a particular point within the Netherlands. The raw data is provided in a 2Gb zip file full of XML files. We proceed in two phases

### phase 1: preparation

We construct a bounding box for each postcode. That makes approximate checking for whether a point is close to a postcode very fast. For improved accuracy, we also store the actual points for each postcode.

NOTE: There are also some postcodes like `9999ZZ` or `9999AA` that seem to be used as placeholders. Those don't actually exist, but they do occur in the data and span most of the country.

### phase 2: retrieval

We draw a bounding box around our target point, then loop over all postcode bounding boxes and retain those that intersect with our target.
Then for each retained postcode, we check that the target point is close enough to an actual adres within the postcode.

## Preparing the data

We use data from `inspireadressen.zip`:

```shell
wget -q -O /data/inspireadressen.zip http://geodata.nationaalgeoregister.nl/inspireadressen/extract/inspireadressen.zip
```

In particular we need two files:

* `9999VBO08102021.zip` has data about Verblijfsobjecten
* `9999NUM08102021.zip` has data about Nummeraanduidingen

A `Verblijfsobject` has a location (usually a point, sometimes a polygon. In the polygon case, we use the centroid of the polygon) and a key into the `Nummeraanduiding`en. A `Nummeraanduiding` has a postcode. We parse the two files to get big arrays of both of these data types. Then we create a big array of a size big enough that we can use a postcode as an index (see below), initialize each element with an infinite bounding box.

For each `Verblijfsobject`, we find its postcode, use that as an index into our bounding boxes array, and extend the relevant bounding box with the point of the current `Verblijfsobject`.

Separately, we also keep track of an array of points: again we use a postcode as the index, but this time the elements are vectors of points.

### Storing the data

Parsing the files and building the big arrays is expensive. We'd like to do it only once and save the state to disk, then load this already-processed data when a request comes in.

When loading the data, we don't want any parsing overhead. This property is provided by the `mmap2` (memory map, version 2) syscall. It maps a file into memory, concretely meaning we can treat a file as a slice of bytes, without the whole file having to be loaded into RAM. If we make sure that it is safe to cast this `&[u8]` to a `&[T]` for the `T` that we want, then the cost of loading the data is effectively zero. An important detail is that `mmap2` guarantees alignment to a page boundary (on 64-bit systems, that means it's 16-bit aligned).

The bounding boxes are stored as-is (taking 16 bytes per bounding box). For the points, we store two arrays. One is an array of actual 2D points (taking 8 bytes per element), the other is indexed by a postcode, and contains `(start_index, length)` pairs. Effectively it's an array of slices into the points array.

## Encoding a Postcode

A postcode is four digits followed by two uppercase letters.

```
9999ZZ
```

We represent this at runtime as a `u16` for the digits, and an array `[ 2; u8 ]` for the letters.
However, we'd like to use a postcode as an index. The current representation is 32 bits, but creating a
`Vec<_>` of `u32::MAX` elements failed on my system. We can represent the data more compactly using only 24 bits:

- the highest number we represent is 9999, that only needs 14 bits, not 16
- each letter only has 26 possibilities, requiring 5 bits each

That adds up to an neat 24 bits per element. A vector of size `2 ** 24` is no problem on my system.

## What is distance

Currently, euclidian distance is used. It is fast and within the borders of the Netherlands it should be accurate enough for our purposes (curvature of the earth should not matter).

