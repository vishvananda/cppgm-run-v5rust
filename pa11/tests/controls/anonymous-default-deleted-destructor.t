// expect reject
struct Value { ~Value() = delete; };
struct Holder {
  struct { Value value; };
};
int main() {
  Holder object;
  return 0;
}
