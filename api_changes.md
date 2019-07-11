- [ ] Client no longer maintains a reference to the monitor it is on
  - This results in horrible circular references to everything which the borrow
  checker is NOT happy about. The monitor will need to be looked up or explicitly
  passed in.
    - Passing in explicitly for now but that may need to change.
