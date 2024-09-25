Rust version:

~/src/wdd/rust/rup/struct_test[pyport]$ time target/release/struct_test test/python.text 100000
Creating random list of words... took 27.819584ms
Sorting words... took 17.341834ms
Removing duplicates... took 223.958µs
Writing words to file... took 4.008041ms
Reading words from file... took 6.926833ms
Inserting 100 words while present 1 ... took 13.542µs
Removing 100 words 2 ... took 5.920667ms
Inserting 100 words back in 1 ... took 9.649291ms
Reversing words... took 64.167µs
Writing sorted words back to file... took 4.255167ms
Total time to complete test: 28.811959ms

real    0m0.582s
user    0m0.079s
sys     0m0.010s


Python version:

~/src/wdd/rust/rup/struct_test[pyport]$ time python main.py test/python.text 100000
Creating random list of words... took 121.785625 ms
Sorting words... took 20.491833 ms
Removing duplicates... took 6.806667 ms
Writing words to file... took 9.077166 ms
Reading words from file... took 9.672292 ms
Inserting 100 words while present 1 ... took 0.036666 ms
Removing 100 words 2 ... took 1.555208 ms
Inserting 100 words back in 1 ... took 52.010583 ms
Reversing words... took 0.026000 ms
Writing sorted words back to file... took 8.684792 ms
Total time to complete test: 45.681584 ms

real    0m0.314s
user    0m0.229s
sys     0m0.022s