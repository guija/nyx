# Nyx: Revolutionizing Flight Dynamics

**Blazing fast from mission concept to operations, and automation.** -- [https://nyxspace.com/](https://nyxspace.com/)

Nyx is provided under the AGPLv3 License. By using this software, you assume responsibility for adhering to the license. Refer to [the pricing page](https://nyxspace.com/pricing/) for an FAQ on the AGPLv3 license.

[![nyx-space on crates.io][cratesio-image]][cratesio]
[![nyx-space on docs.rs][docsrs-image]][docsrs]
[![LoC](https://tokei.rs/b1/github/nyx-space/nyx?category=lines)](https://github.com/nyx-space/nyx).
[![codecov](https://codecov.io/gh/nyx-space/nyx/graph/badge.svg?token=gEiAvwzwh5)](https://codecov.io/gh/nyx-space/nyx)

[cratesio-image]: https://img.shields.io/crates/v/nyx-space.svg
[cratesio]: https://crates.io/crates/nyx-space
[docsrs-image]: https://docs.rs/nyx-space/badge.svg
[docsrs]: https://docs.rs/nyx-space/

## Who am I?
An GNC and flight dynamics engineer with a heavy background in software. I currently work for Rocket Lab USA on the Blue Ghost lunar lander. -- Find me on [LinkedIn](https://www.linkedin.com/in/chrisrabotin/).

# Run performance tests

```
rustup override set nightly
rustup update
cargo test --release performance_test -- --nocapture
```

# Technical performance results with different compiler optimization settings

Inputs:
- Same reference frame (eme2000)

```
codegen-units = 1
lto = "fat"
#rustflags = ["-Z", "threads=8"]
panic = "abort"
debug-assertions = false
overflow-checks = false
# try also 2, may be faster
opt-level = 3
debug = false
```

stable-x86_64-unknown-linux-gnu (directory override for '/home/guillermo/git/nyx-guillermo')
rustc 1.78.0 (9b00956e5 2024-04-29)

Compilation duration = ?
mean = 105.03288320000001 ms
median = 92.72473600000001 ms

---

Inputs:
- Same reference frame (eme2000)

```
codegen-units = 1
lto = "fat"
#rustflags = ["-Z", "threads=8"]
panic = "abort"
debug-assertions = false
overflow-checks = false
# try also 2, may be faster
opt-level = 3
debug = false
```

nightly-x86_64-unknown-linux-gnu (directory override for '/home/guillermo/git/nyx-guillermo')
rustc 1.80.0-nightly (b1ec1bd65 2024-05-18)

Compilation duration (all dependencies = 7 mins)
mean = 87.24208639999999 ms
median = 87.21664000000001 ms

---

Inputs:
- Same reference frame (eme2000)

```
[profile.release]
codegen-units = 1
lto = false
#rustflags = ["-Z", "threads=8"]
panic = "abort"
debug-assertions = false
overflow-checks = false
# try also 2, may be faster
opt-level = 3
debug = false
```

nightly-x86_64-unknown-linux-gnu (directory override for '/home/guillermo/git/nyx-guillermo')
rustc 1.80.0-nightly (b1ec1bd65 2024-05-18)

Compilation duration (all dependencies = 6 mins)
mean = 91.4187008 ms
median = 91.107584 ms

---

Inputs:
- Different reference frames. Orbit IAU earth, Harmonics: EME2000

```
[profile.release]
codegen-units = 1
lto = false
#rustflags = ["-Z", "threads=8"]
panic = "abort"
debug-assertions = false
overflow-checks = false
# try also 2, may be faster
opt-level = 3
debug = false
```

nightly-x86_64-unknown-linux-gnu (directory override for '/home/guillermo/git/nyx-guillermo')
rustc 1.80.0-nightly (b1ec1bd65 2024-05-18)

mean = 314.7364096 ms
median = 303.06688 ms

# Orekit performance test

Same reference frame

mean = 165.9 ms
median = 119.0 ms

---

Different reference frames

mean = 310.6 ms
median = 221.0 ms

# Results summary

- For same frames: 87ms (nyx) vs 119ms (orekit) -> nyx is 27% faster.
- For different frames: 303ms vs 221ms (orekit) -> nys is 37% slower.
- Reference frame transformations are significantly faster in orekit or the harmonics implementation is very different in orekit.
  - Potentially slow candidate from profiling: `rotations.rs::dcm_to_parent`

# Performance monitoring

```
cargo install flamegraph
sudo apt install linux-tools-common linux-tools-generic linux-tools-`uname -r`
sudo sysctl kernel.perf_event_paranoid=3
cargo flamegraph --release --test lib -- propagation::trajectory::performance_test
sudo apt install hotspot # install ui for opening perf traces
hotspot perf.data
````

