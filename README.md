# Page Expander

Small tool which converts the contents of a Ghost blog backup to a static page.

## Build

This is a [Rust](https://www.rust-lang.org) project.

Build and run using cargo.

## Usage

`$ ./page-expander "/path/to/ghost/blog/content" "/path/to/the/templates/folder/*.html" "/path/to/output/folder"`

* The ghost blog content export should contain a json back-up of the database in the `data` folder.

