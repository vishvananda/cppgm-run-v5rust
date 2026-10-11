// N3485 [dcl.init.aggr]/4 forbids an empty initializer for an unknown bound.
int values[] = {};
int main() { return 0; }
