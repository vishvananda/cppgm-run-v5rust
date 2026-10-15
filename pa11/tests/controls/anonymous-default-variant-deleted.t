// expect reject
struct Value { Value() {} };
struct Holder {
  union { Value value; int other; };
};
int main() {
  Holder object;
  return 0;
}
