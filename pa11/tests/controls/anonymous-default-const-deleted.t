// expect reject
struct Holder {
  struct { const int value; };
};
int main() {
  Holder object;
  return 0;
}
