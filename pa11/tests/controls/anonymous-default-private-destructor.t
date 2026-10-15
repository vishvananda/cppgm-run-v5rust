// expect reject
class Value { ~Value() {} };
struct Holder {
  struct { Value value; };
};
int main() {
  Holder object;
  return 0;
}
