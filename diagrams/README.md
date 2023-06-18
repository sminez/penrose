# Diagrams

Assorted diagrams for use as reference in understanding the structure and logic
of the code within Penrose.


### Regenerating the svgs

The diagram files themselves are written in the graphviz [dot](https://graphviz.org/doc/info/lang.html)
language. To output an svg from a given dot file, make sure that you have graphviz installed and then
run the following:
```sh
$ FNAME="state" dot -Tsvg -o $FNAME.svg $FNAME.dot
```
