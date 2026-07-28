# SQLite Database Reader & Query Engine

## Overview

A SQLite database reader and query engine implemented in Rust. It parses SQLite's database format, traverses table and index B-trees, decodes SQLite records, and executes a subset of SQL queries. The purpose is to understand the structure and architecture of an SQLite database based on the official [SQLite documentation](https://sqlite.org/fileformat2.html).

------

## Features

### Database Commands

```
.dbinfo
.tables
.schema
.schema table_name
.indexes
```

`.dbinfo` Displays the number of tables and page size

`.tables` Lists tables from the `sqlite_schema`

`.schema` Lists the `CREATE` DDL statement text used to establish the object

`.indexes` list the names of all indices

### SQL Queries

Currently supported:

```
SELECT * FROM table;
SELECT COUNT (*) FROM table;
SELECT column_a, column_b FROM table;
SELECT column_a, column_b FROM table WHERE column_c = 'value';
```

### Index Support

If an index for a `WHERE` query exists, the search will traverse the index B-tree and collect the `row_id` for every valid row, then use the `row_id` to traverse the table B-tree and gather the rows. 

For example, if we want to see the indexes available for the `chinook.db` database, we can run `cargo r -- databases/chinook.db .indexes"`

This will output: 

```rust
ArtistId: albums
SupportRepId: customers
ReportsTo: employees
CustomerId: invoices
InvoiceId: invoice_items
TrackId: invoice_items
TrackId: playlist_track
AlbumId: tracks
GenreId: tracks
MediaTypeId: tracks
```

We can see that the `ArtistId` is indexed in the table `albums`.

If we want to know what columns are available for that table, we can use `cargo r -- databases/chinook.db .schema albums`:

```rust
CREATE TABLE "albums"
(
    [AlbumId] INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    [Title] NVARCHAR(160)  NOT NULL,
    [ArtistId] INTEGER  NOT NULL,
    FOREIGN KEY ([ArtistId]) REFERENCES "artists" ([ArtistId]) 
                ON DELETE NO ACTION ON UPDATE NO ACTION
)
```

Here we see that the columns are `AlbumId`, `Title`,  and `ArtistId`. Now to search for a title using an `ArtistId`, we can run `cargo r -- databases/chinook.db "SELECT Title FROM albums WHERE ArtistId = '216'"`:

```rust
Mozart: Wind Concertos
```

This indexed query would be much faster than say, using the AlbumID (which is not indexed for albums) `cargo r -- databases/chinook.db "SELECT Title FROM albums WHERE AlbumId = '282'"`

When an index doesn't exist, the search performs a full table scan collecting every row from the table, and then filtering valid rows. This has a higher time complexity of $O(N)$  versus the index search time complexity of $O(\log(N+K)$ , where $N$ is the number of rows and $K$ is the number of rows matching the query.

------

## Example Usage

Command and query examples

```
cargo run -- databases/chinook.db .dbinfo
cargo run -- databases/chinook.db .tables
cargo run -- databases/chinook.db "SELECT COUNT(*) FROM artists"
cargo run -- databases/chinook.db "SELECT FirstName, LastName, Email FROM customers"
cargo run -- databases/chinook.db "SELECT name, color FROM apples WHERE color = 'Yellow'"
```

```
$ cargo run -- databases/chinook.db .tables

albums
sqlite_sequence
artists
customers
employees
genres
invoices
invoice_items
media_types
playlists
playlist_track
tracks
sqlite_stat1
```

The command-line format:

```
<program> <database path> <command>
```

------

## Limitations / Supported SQL

Support is currently limited to only reading databases.

```
Supported:
- SELECT
- SELECT COUNT(*)
- WHERE equality conditions
- Column selection
- Table listing
- Schema listing
- Database information
- Index-assisted WHERE queries
- Full table scans
```

Potential future work:

```
- INSERT
- UPDATE
- DELETE
- JOIN
- ORDER BY
- GROUP BY
- Aggregate functions
- Multiple WHERE conditions
- More comparison operators
- More complete SQL parser
- Transactions
- Database writes
```
