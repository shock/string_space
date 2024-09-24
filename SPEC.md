# Word Test Specification

## Overview

The word test is a simple rust program that tests how quick a list of words can be read from a file, sorted, reversed, and written back to the same file.  The file will be specified as a command line argument.  The program will time how long it takes to read the file, sort the words, remove any duplicates, reverse the list, and write the sorted list back to the file.  It will then print the time it took to do each of these operations, as well as the total time it took to complete the test.

## Command Line Arguments

- --file: The file to read and write the sorted list to.
- --num-words: The number of words in the file.

## Test Data

When the program is run, it will create a random list of N words, where N is the number of words specified by the --num-words argument.  The words will be written to a file specified by the --file argument.  Words can be any sequence of letters and numbers, with a random length between 3 and 20 characters.  Once the words are written to the file, the memory of the random words should be freed.

## Test Operations

Once the file is created, the program will perform the following operations:

- Read the file and store the words in Vector<String>.
- Sort the Vector<String> in ascending order.
- Remove duplicates from the Vector<String>.
- Reverse the Vector<String>.
- Write the Vector<String> back to the file.

Each of these operations will be be implemented as a separate function in the program.
A test harness will then call each of these functions and measure the time it takes to complete each operation.

The program will run each test and print the time it took to complete each operation.
The program will then print the total time it took to complete the test.

## Test Output

When the program is run, it will print the following output:

```
Creating random list of words...
Reading words from file...
Sorting words...
Removing duplicates...
Reversing words...
Writing sorted words back to file...

Reading words from file took: 0.00 seconds
Sorting words took: 0.00 seconds
Removing duplicates took: 0.00 seconds
Reversing words took: 0.00 seconds
Writing sorted words back to file took: 0.00 seconds
Total time to complete test: 0.00 seconds
```

The program will print the time it took to complete each operation, as well as the total time it took to complete the test.
