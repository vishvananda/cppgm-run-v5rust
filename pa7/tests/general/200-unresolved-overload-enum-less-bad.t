int overloaded(int);
int overloaded(long);
enum Value { item };
void probe(Value value) {
  overloaded < value;
}
