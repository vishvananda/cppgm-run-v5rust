int overloaded(int value) { return value; }
long overloaded(long value) { return value; }
struct Value { int number; };
int operator+(int (*function)(int), const Value& value) { return function(value.number); }
int operator+(const Value& value, int (*function)(int)) { return function(value.number); }
int probe(const Value& value) {
  return (overloaded + value) + (value + overloaded);
}
