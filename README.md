![License: MIT][s1]

[s1]: https://img.shields.io/badge/License-MIT-blue.svg

# Noteshrink-rs

This is a Rust rewrite of Matt Zucker's [noteshrink](https://github.com/mzucker/noteshrink) which converts scans of handwritten notes to beautiful, compact PDFs. For its inner-workings see this great [writeup](https://mzucker.github.io/2016/09/20/noteshrink.html) for more details.

<p align="center" width="100%">
  <img width="33%" src="https://github.com/SingularHomology/noteshrink-rs/blob/main/examples/notesA1.jpg?raw=true"/>
  <img width="33%" src="https://github.com/SingularHomology/noteshrink-rs/blob/main/examples/notesA1-output.png?raw=true"/>
</p>

<p align="center">
  <img src="https://github.com/SingularHomology/noteshrink-rs/blob/main/tty.gif?raw=true"/>
</p>

# Current status

Most of noteshrink's original functionality is implemented. PDF support isn't there yet, all outputs are image only. The implementation currently relies on the [ndarray](https://github.com/rust-ndarray/ndarray) and [kmeans](https://github.com/seijikun/kmean-rs) crates. The latter offers measurable performance improvements over the original python implementation by leveraging SIMD capabilities. Note that you need a nightly compiler to be able to make it work. 

The long term goal is to hopefully improve its performance even more and add some new features. 

# Building from source

```
git clone https://github.com/SingularHomology/noteshrink-rs.git
cd noteshrink-rs
cargo build --release
```
