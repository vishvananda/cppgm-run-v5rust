// expect reject
struct Holder {
  struct { int& value; };
};
int main() {
  Holder object;
  return 0;
}
