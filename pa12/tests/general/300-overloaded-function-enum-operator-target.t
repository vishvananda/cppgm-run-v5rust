int overloaded(int value) { return value; }
long overloaded(long value) { return value; }
enum Value { item };
int operator+(int (*function)(int), Value value) { return function(value); }
int operator+(Value value, int (*function)(int)) { return function(value); }
int probe(Value value) {
  return (overloaded + value) + (value + overloaded);
}
