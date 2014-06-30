# Rust `ringbuf`

An implementation of Rust's `RingBuf` based on `Vec`.

### Pros
1. Performance is about 30-40% better for reads and writes than the standard library ringbuf.  See benchmark output below for specifics.
2. Provides methods that would be impossible to implement (efficiently) for the current implementation including `as_slices`, `into_vec`, and `from_vec`.
3. More complete method documentation including more examples.
4. A `move_iter` implementation.

### Cons
1. Based on `Vec`, so a lot of `unsafe`.


## Benchmarks

```
estimating the cost of precise_time_ns()
> warming up for 1000 ms
> collecting 100 measurements, 335544 iters each in estimated 1.7102 s
> found 11 outliers among 100 measurements (11.00%)
  > 1 (1.00%) low mild
  > 2 (2.00%) high mild
  > 8 (8.00%) high severe
> estimating statistics
  > bootstrapping sample with 100000 resamples
  > mean   50.403 ns ± 55.466 ps [50.295 ns 50.513 ns] 95% CI
  > median 50.369 ns ± 48.989 ps [50.315 ns 50.493 ns] 95% CI
  > MAD    470.54 ps ± 53.976 ps [345.07 ps 561.41 ps] 95% CI
  > SD     535.49 ps ± 44.402 ps [443.33 ps 616.77 ps] 95% CI

benchmarking push_back_default_allocate_8
> warming up for 1000 ms
> collecting 100 measurements, 20971 iters each in estimated 1.8115 s
> found 10 outliers among 100 measurements (10.00%)
  > 2 (2.00%) high mild
  > 8 (8.00%) high severe
> estimating statistics
  > bootstrapping sample with 100000 resamples
  > mean   856.86 ns ± 906.72 ps [855.14 ns 858.69 ns] 95% CI
  > median 855.67 ns ± 985.54 ps [853.82 ns 857.35 ns] 95% CI
  > MAD    8.4328 ns ± 1.1946 ns [5.2960 ns 9.9901 ns] 95% CI
  > SD     8.7372 ns ± 888.66 ps [6.9363 ns 10.411 ns] 95% CI
> comparing with previous sample
  > bootstrapping sample with 100000 resamples
  > mean   -29.744% ± 0.1124% [-29.964% -29.523%] 95% CI
  > median -29.697% ± 0.1091% [-29.945% -29.512%] 95% CI

benchmarking push_back_default_allocate_128
> warming up for 1000 ms
> collecting 100 measurements, 1310 iters each in estimated 1.2442 s
> found 4 outliers among 100 measurements (4.00%)
  > 4 (4.00%) high severe
> estimating statistics
  > bootstrapping sample with 100000 resamples
  > mean   9.2768 us ± 9.8299 ns [9.2577 us 9.2963 us] 95% CI
  > median 9.2714 us ± 14.457 ns [9.2413 us 9.3001 us] 95% CI
  > MAD    105.53 ns ± 12.229 ns [78.483 ns 126.81 ns] 95% CI
  > SD     96.969 ns ± 6.2933 ns [83.992 ns 108.71 ns] 95% CI
> comparing with previous sample
  > bootstrapping sample with 100000 resamples
  > mean   -35.374% ± 0.1012% [-35.572% -35.176%] 95% CI
  > median -35.362% ± 0.1383% [-35.629% -35.082%] 95% CI

benchmarking push_back_default_allocate_1024
> warming up for 1000 ms
> collecting 100 measurements, 163 iters each in estimated 1.0626 s
> found 14 outliers among 100 measurements (14.00%)
  > 2 (2.00%) high mild
  > 12 (12.00%) high severe
> estimating statistics
  > bootstrapping sample with 100000 resamples
  > mean   64.840 us ± 72.586 ns [64.704 us 64.986 us] 95% CI
  > median 64.752 us ± 76.414 ns [64.605 us 64.892 us] 95% CI
  > MAD    544.83 ns ± 87.553 ns [398.77 ns 722.18 ns] 95% CI
  > SD     685.29 ns ± 84.344 ns [519.48 ns 845.54 ns] 95% CI
> comparing with previous sample
  > bootstrapping sample with 100000 resamples
  > mean   -36.109% ± 0.0934% [-36.291% -35.925%] 95% CI
  > median -36.144% ± 0.0856% [-36.311% -35.977%] 95% CI

benchmarking push_back_default_allocate_32768
> warming up for 1000 ms
> collecting 100 measurements, 5 iters each in estimated 1.0529 s
> found 6 outliers among 100 measurements (6.00%)
  > 3 (3.00%) high mild
  > 3 (3.00%) high severe
> estimating statistics
  > bootstrapping sample with 100000 resamples
  > mean   2.0594 ms ± 2.1316 us [2.0553 ms 2.0636 ms] 95% CI
  > median 2.0561 ms ± 2.1837 us [2.0530 ms 2.0628 ms] 95% CI
  > MAD    16.965 us ± 2.6209 us [13.800 us 24.284 us] 95% CI
  > SD     21.046 us ± 1.4734 us [17.953 us 23.715 us] 95% CI
> comparing with previous sample
  > bootstrapping sample with 100000 resamples
  > mean   -35.957% ± 0.1273% [-36.212% -35.713%] 95% CI
  > median -35.894% ± 0.1186% [-36.089% -35.625%] 95% CI

benchmarking push_back_pre_allocate_8
> warming up for 1000 ms
> collecting 100 measurements, 20971 iters each in estimated 1.8242 s
> found 8 outliers among 100 measurements (8.00%)
  > 2 (2.00%) high mild
  > 6 (6.00%) high severe
> estimating statistics
  > bootstrapping sample with 100000 resamples
  > mean   856.00 ns ± 683.05 ps [854.69 ns 857.36 ns] 95% CI
  > median 855.16 ns ± 737.31 ps [853.67 ns 856.73 ns] 95% CI
  > MAD    5.3028 ns ± 647.91 ps [4.1438 ns 6.6736 ns] 95% CI
  > SD     6.6546 ns ± 625.32 ps [5.3882 ns 7.8333 ns] 95% CI
> comparing with previous sample
  > bootstrapping sample with 100000 resamples
  > mean   -29.480% ± 0.1033% [-29.686% -29.281%] 95% CI
  > median -29.443% ± 0.1065% [-29.652% -29.238%] 95% CI

benchmarking push_back_pre_allocate_128
> warming up for 1000 ms
> collecting 100 measurements, 1310 iters each in estimated 1.0913 s
> found 8 outliers among 100 measurements (8.00%)
  > 2 (2.00%) high mild
  > 6 (6.00%) high severe
> estimating statistics
  > bootstrapping sample with 100000 resamples
  > mean   8.1763 us ± 11.676 ns [8.1541 us 8.1998 us] 95% CI
  > median 8.1600 us ± 12.012 ns [8.1349 us 8.1798 us] 95% CI
  > MAD    109.16 ns ± 12.700 ns [81.956 ns 131.40 ns] 95% CI
  > SD     114.03 ns ± 11.910 ns [90.250 ns 136.50 ns] 95% CI
> comparing with previous sample
  > bootstrapping sample with 100000 resamples
  > mean   -41.703% ± 0.1119% [-41.921% -41.483%] 95% CI
  > median -41.669% ± 0.1162% [-41.902% -41.452%] 95% CI

benchmarking push_back_pre_allocate_1024
> warming up for 1000 ms
> collecting 100 measurements, 163 iters each in estimated 1.0219 s
> found 17 outliers among 100 measurements (17.00%)
  > 5 (5.00%) high mild
  > 12 (12.00%) high severe
> estimating statistics
  > bootstrapping sample with 100000 resamples
  > mean   63.137 us ± 146.28 ns [62.868 us 63.439 us] 95% CI
  > median 62.770 us ± 146.67 ns [62.594 us 63.094 us] 95% CI
  > MAD    872.60 ns ± 129.55 ns [653.51 ns 1.1627 us] 95% CI
  > SD     1.3735 us ± 195.53 ns [964.10 ns 1.7248 us] 95% CI
> comparing with previous sample
  > bootstrapping sample with 100000 resamples
  > mean   -41.972% ± 0.1446% [-42.243% -41.677%] 95% CI
  > median -42.243% ± 0.1443% [-42.438% -41.918%] 95% CI

benchmarking push_back_pre_allocate_32768
> warming up for 1000 ms
> collecting 100 measurements, 5 iters each in estimated 1.0524 s
> found 12 outliers among 100 measurements (12.00%)
  > 2 (2.00%) high mild
  > 10 (10.00%) high severe
> estimating statistics
  > bootstrapping sample with 100000 resamples
  > mean   2.0437 ms ± 2.6774 us [2.0386 ms 2.0491 ms] 95% CI
  > median 2.0419 ms ± 3.1553 us [2.0345 ms 2.0494 ms] 95% CI
  > MAD    26.765 us ± 2.3945 us [20.177 us 29.701 us] 95% CI
  > SD     25.414 us ± 2.7201 us [20.055 us 30.654 us] 95% CI
> comparing with previous sample
  > bootstrapping sample with 100000 resamples
  > mean   -42.696% ± 0.1251% [-42.946% -42.453%] 95% CI
  > median -42.653% ± 0.1892% [-42.960% -42.291%] 95% CI

benchmarking push_pre_default_allocate_8
> warming up for 1000 ms
> collecting 100 measurements, 20971 iters each in estimated 1.7712 s
> found 11 outliers among 100 measurements (11.00%)
  > 11 (11.00%) high severe
> estimating statistics
  > bootstrapping sample with 100000 resamples
  > mean   838.03 ns ± 1.0483 ns [836.00 ns 840.12 ns] 95% CI
  > median 836.31 ns ± 1.0983 ns [834.45 ns 838.52 ns] 95% CI
  > MAD    9.6323 ns ± 1.2099 ns [6.9853 ns 11.910 ns] 95% CI
  > SD     9.9419 ns ± 798.97 ps [8.3388 ns 11.467 ns] 95% CI
> comparing with previous sample
  > bootstrapping sample with 100000 resamples
  > mean   -35.500% ± 0.9300% [-37.323% -33.675%] 95% CI
  > median -29.171% ± 0.7712% [-30.516% -28.757%] 95% CI

benchmarking push_pre_default_allocate_128
> warming up for 1000 ms
> collecting 100 measurements, 1310 iters each in estimated 1.1839 s
> found 12 outliers among 100 measurements (12.00%)
  > 4 (4.00%) high mild
  > 8 (8.00%) high severe
> estimating statistics
  > bootstrapping sample with 100000 resamples
  > mean   8.8914 us ± 14.730 ns [8.8638 us 8.9214 us] 95% CI
  > median 8.8515 us ± 10.454 ns [8.8320 us 8.8697 us] 95% CI
  > MAD    87.250 ns ± 15.408 ns [64.221 ns 124.24 ns] 95% CI
  > SD     141.36 ns ± 15.640 ns [108.16 ns 169.42 ns] 95% CI
> comparing with previous sample
  > bootstrapping sample with 100000 resamples
  > mean   -44.414% ± 0.1156% [-44.636% -44.184%] 95% CI
  > median -44.556% ± 0.0937% [-44.755% -44.396%] 95% CI

benchmarking push_pre_default_allocate_1024
> warming up for 1000 ms
> collecting 100 measurements, 163 iters each in estimated 1.0061 s
> found 5 outliers among 100 measurements (5.00%)
  > 5 (5.00%) high severe
> estimating statistics
  > bootstrapping sample with 100000 resamples
  > mean   61.513 us ± 56.776 ns [61.403 us 61.625 us] 95% CI
  > median 61.443 us ± 75.420 ns [61.336 us 61.584 us] 95% CI
  > MAD    569.28 ns ± 80.452 ns [399.83 ns 718.11 ns] 95% CI
  > SD     555.96 ns ± 35.933 ns [480.61 ns 621.48 ns] 95% CI
> comparing with previous sample
  > bootstrapping sample with 100000 resamples
  > mean   -47.342% ± 0.0774% [-47.493% -47.190%] 95% CI
  > median -47.329% ± 0.0759% [-47.477% -47.186%] 95% CI

benchmarking push_pre_default_allocate_32768
> warming up for 1000 ms
> collecting 100 measurements, 5 iters each in estimated 978.15 ms
> found 2 outliers among 100 measurements (2.00%)
  > 1 (1.00%) high mild
  > 1 (1.00%) high severe
> estimating statistics
  > bootstrapping sample with 100000 resamples
  > mean   1.9499 ms ± 2.2085 us [1.9456 ms 1.9543 ms] 95% CI
  > median 1.9478 ms ± 3.1834 us [1.9428 ms 1.9546 ms] 95% CI
  > MAD    21.037 us ± 2.6943 us [16.149 us 26.713 us] 95% CI
  > SD     22.189 us ± 1.7682 us [18.684 us 25.592 us] 95% CI
> comparing with previous sample
  > bootstrapping sample with 100000 resamples
  > mean   -47.959% ± 0.1167% [-48.194% -47.737%] 95% CI
  > median -47.721% ± 0.1037% [-47.879% -47.480%] 95% CI

benchmarking push_pre_allocate_8
> warming up for 1000 ms
> collecting 100 measurements, 20971 iters each in estimated 1.7962 s
> found 6 outliers among 100 measurements (6.00%)
  > 1 (1.00%) high mild
  > 5 (5.00%) high severe
> estimating statistics
  > bootstrapping sample with 100000 resamples
  > mean   849.93 ns ± 748.97 ps [848.49 ns 851.44 ns] 95% CI
  > median 849.69 ns ± 798.54 ps [848.14 ns 851.52 ns] 95% CI
  > MAD    7.1378 ns ± 745.80 ps [5.3836 ns 8.3104 ns] 95% CI
  > SD     7.3456 ns ± 784.56 ps [5.8689 ns 8.8943 ns] 95% CI
> comparing with previous sample
  > bootstrapping sample with 100000 resamples
  > mean   -27.701% ± 0.1079% [-27.916% -27.492%] 95% CI
  > median -27.487% ± 0.1152% [-27.726% -27.250%] 95% CI

benchmarking push_pre_allocate_128
> warming up for 1000 ms
> collecting 100 measurements, 1310 iters each in estimated 1.0532 s
> found 8 outliers among 100 measurements (8.00%)
  > 8 (8.00%) high severe
> estimating statistics
  > bootstrapping sample with 100000 resamples
  > mean   8.0365 us ± 10.426 ns [8.0166 us 8.0574 us] 95% CI
  > median 8.0264 us ± 15.490 ns [7.9981 us 8.0528 us] 95% CI
  > MAD    89.726 ns ± 11.028 ns [71.205 ns 114.08 ns] 95% CI
  > SD     100.50 ns ± 8.7658 ns [82.255 ns 116.63 ns] 95% CI
> comparing with previous sample
  > bootstrapping sample with 100000 resamples
  > mean   -40.115% ± 0.0961% [-40.301% -39.924%] 95% CI
  > median -40.187% ± 0.1295% [-40.462% -39.975%] 95% CI

benchmarking push_pre_allocate_1024
> warming up for 1000 ms
> collecting 100 measurements, 163 iters each in estimated 1.0118 s
> found 5 outliers among 100 measurements (5.00%)
  > 3 (3.00%) high mild
  > 2 (2.00%) high severe
> estimating statistics
  > bootstrapping sample with 100000 resamples
  > mean   61.516 us ± 90.502 ns [61.342 us 61.697 us] 95% CI
  > median 61.467 us ± 105.91 ns [61.286 us 61.693 us] 95% CI
  > MAD    813.36 ns ± 100.51 ns [635.88 ns 1.0369 us] 95% CI
  > SD     897.16 ns ± 77.564 ns [740.41 ns 1.0438 us] 95% CI
> comparing with previous sample
  > bootstrapping sample with 100000 resamples
  > mean   -41.286% ± 0.1106% [-41.502% -41.068%] 95% CI
  > median -41.169% ± 0.1185% [-41.400% -40.940%] 95% CI

benchmarking push_pre_allocate_32768
> warming up for 1000 ms
> collecting 100 measurements, 5 iters each in estimated 997.64 ms
> found 9 outliers among 100 measurements (9.00%)
  > 2 (2.00%) high mild
  > 7 (7.00%) high severe
> estimating statistics
  > bootstrapping sample with 100000 resamples
  > mean   1.9671 ms ± 2.6933 us [1.9621 ms 1.9726 ms] 95% CI
  > median 1.9607 ms ± 1.9773 us [1.9572 ms 1.9636 ms] 95% CI
  > MAD    19.118 us ± 3.7562 us [13.082 us 27.168 us] 95% CI
  > SD     26.088 us ± 2.6465 us [20.732 us 31.042 us] 95% CI
> comparing with previous sample
  > bootstrapping sample with 100000 resamples
  > mean   -42.403% ± 0.1104% [-42.620% -42.187%] 95% CI
  > median -42.447% ± 0.0895% [-42.624% -42.269%] 95% CI

benchmarking iterate_8
> warming up for 1000 ms
> collecting 100 measurements, 41943 iters each in estimated 1.4924 s
> found 9 outliers among 100 measurements (9.00%)
  > 2 (2.00%) high mild
  > 7 (7.00%) high severe
> estimating statistics
  > bootstrapping sample with 100000 resamples
  > mean   353.15 ns ± 578.77 ps [352.06 ns 354.33 ns] 95% CI
  > median 351.91 ns ± 559.22 ps [350.95 ns 352.76 ns] 95% CI
  > MAD    4.6658 ns ± 738.20 ps [2.9788 ns 5.7573 ns] 95% CI
  > SD     5.5902 ns ± 587.38 ps [4.3998 ns 6.6908 ns] 95% CI
> comparing with previous sample
  > bootstrapping sample with 100000 resamples
  > mean   +2.3547% ± 0.1884% [+1.9940% +2.7341%] 95% CI
  > median +1.9942% ± 0.2036% [+1.6405% +2.4267%] 95% CI

benchmarking iterate_128
> warming up for 1000 ms
> collecting 100 measurements, 5242 iters each in estimated 1.6882 s
> found 13 outliers among 100 measurements (13.00%)
  > 2 (2.00%) high mild
  > 11 (11.00%) high severe
> estimating statistics
  > bootstrapping sample with 100000 resamples
  > mean   3.1724 us ± 3.8852 ns [3.1650 us 3.1802 us] 95% CI
  > median 3.1677 us ± 3.5253 ns [3.1592 us 3.1742 us] 95% CI
  > MAD    28.529 ns ± 3.9378 ns [22.564 ns 38.221 ns] 95% CI
  > SD     36.833 ns ± 3.7119 ns [29.057 ns 43.569 ns] 95% CI
> comparing with previous sample
  > bootstrapping sample with 100000 resamples
  > mean   -32.075% ± 0.1210% [-32.311% -31.837%] 95% CI
  > median -32.131% ± 0.1521% [-32.410% -31.833%] 95% CI

benchmarking iterate_1024
> warming up for 1000 ms
> collecting 100 measurements, 655 iters each in estimated 1.6058 s
> found 11 outliers among 100 measurements (11.00%)
  > 2 (2.00%) high mild
  > 9 (9.00%) high severe
> estimating statistics
  > bootstrapping sample with 100000 resamples
  > mean   24.329 us ± 27.063 ns [24.277 us 24.384 us] 95% CI
  > median 24.312 us ± 32.870 ns [24.231 us 24.369 us] 95% CI
  > MAD    201.88 ns ± 27.769 ns [161.94 ns 270.77 ns] 95% CI
  > SD     258.60 ns ± 32.106 ns [196.51 ns 320.50 ns] 95% CI
> comparing with previous sample
  > bootstrapping sample with 100000 resamples
  > mean   -34.323% ± 0.0970% [-34.510% -34.131%] 95% CI
  > median -34.305% ± 0.1213% [-34.600% -34.114%] 95% CI

benchmarking iterate_32768
> warming up for 1000 ms
> collecting 100 measurements, 20 iters each in estimated 1.5508 s
> found 14 outliers among 100 measurements (14.00%)
  > 3 (3.00%) high mild
  > 11 (11.00%) high severe
> estimating statistics
  > bootstrapping sample with 100000 resamples
  > mean   772.71 us ± 926.32 ns [770.95 us 774.58 us] 95% CI
  > median 772.23 us ± 933.93 ns [770.28 us 773.54 us] 95% CI
  > MAD    7.2887 us ± 1.2120 us [4.7887 us 9.3367 us] 95% CI
  > SD     8.7915 us ± 950.45 ns [6.8569 us 10.546 us] 95% CI
> comparing with previous sample
  > bootstrapping sample with 100000 resamples
  > mean   -34.818% ± 0.1236% [-35.063% -34.578%] 95% CI
  > median -34.711% ± 0.1226% [-34.937% -34.458%] 95% CI

benchmarking get_8
> warming up for 1000 ms
> collecting 100 measurements, 41943 iters each in estimated 1.1978 s
> found 5 outliers among 100 measurements (5.00%)
  > 2 (2.00%) high mild
  > 3 (3.00%) high severe
> estimating statistics
  > bootstrapping sample with 100000 resamples
  > mean   282.72 ns ± 299.12 ps [282.14 ns 283.32 ns] 95% CI
  > median 282.45 ns ± 438.43 ps [281.75 ns 283.62 ns] 95% CI
  > MAD    2.8166 ns ± 327.42 ps [2.2051 ns 3.5169 ns] 95% CI
  > SD     2.9633 ns ± 223.78 ps [2.5020 ns 3.3799 ns] 95% CI
> comparing with previous sample
  > bootstrapping sample with 100000 resamples
  > mean   -28.642% ± 0.1108% [-28.860% -28.427%] 95% CI
  > median -28.645% ± 0.1362% [-28.840% -28.307%] 95% CI

benchmarking get_128
> warming up for 1000 ms
> collecting 100 measurements, 5242 iters each in estimated 1.9905 s
> found 9 outliers among 100 measurements (9.00%)
  > 2 (2.00%) high mild
  > 7 (7.00%) high severe
> estimating statistics
  > bootstrapping sample with 100000 resamples
  > mean   3.7536 us ± 3.8439 ns [3.7462 us 3.7613 us] 95% CI
  > median 3.7482 us ± 3.6834 ns [3.7399 us 3.7542 us] 95% CI
  > MAD    29.030 ns ± 4.7876 ns [19.949 ns 38.550 ns] 95% CI
  > SD     37.328 ns ± 3.1821 ns [30.647 ns 43.095 ns] 95% CI
> comparing with previous sample
  > bootstrapping sample with 100000 resamples
  > mean   -32.461% ± 0.0956% [-32.647% -32.273%] 95% CI
  > median -32.533% ± 0.1143% [-32.749% -32.310%] 95% CI

benchmarking get_1024
> warming up for 1000 ms
> collecting 100 measurements, 655 iters each in estimated 1.9655 s
> found 9 outliers among 100 measurements (9.00%)
  > 2 (2.00%) high mild
  > 7 (7.00%) high severe
> estimating statistics
  > bootstrapping sample with 100000 resamples
  > mean   29.656 us ± 29.638 ns [29.600 us 29.717 us] 95% CI
  > median 29.614 us ± 27.432 ns [29.548 us 29.654 us] 95% CI
  > MAD    238.92 ns ± 32.160 ns [179.88 ns 304.81 ns] 95% CI
  > SD     288.01 ns ± 36.588 ns [215.65 ns 357.82 ns] 95% CI
> comparing with previous sample
  > bootstrapping sample with 100000 resamples
  > mean   -32.652% ± 0.0933% [-32.833% -32.468%] 95% CI
  > median -32.691% ± 0.1081% [-32.915% -32.500%] 95% CI

benchmarking get_32768
> warming up for 1000 ms
> collecting 100 measurements, 20 iters each in estimated 1.9070 s
> found 6 outliers among 100 measurements (6.00%)
  > 1 (1.00%) high mild
  > 5 (5.00%) high severe
> estimating statistics
  > bootstrapping sample with 100000 resamples
  > mean   947.02 us ± 1.2645 us [944.62 us 949.58 us] 95% CI
  > median 945.04 us ± 1.0141 us [943.29 us 947.15 us] 95% CI
  > MAD    11.063 us ± 1.4621 us [7.9782 us 13.578 us] 95% CI
  > SD     12.417 us ± 1.3369 us [9.8901 us 15.040 us] 95% CI
> comparing with previous sample
  > bootstrapping sample with 100000 resamples
  > mean   -32.839% ± 0.1221% [-33.077% -32.600%] 95% CI
  > median -32.955% ± 0.1200% [-33.193% -32.697%] 95% CI
```