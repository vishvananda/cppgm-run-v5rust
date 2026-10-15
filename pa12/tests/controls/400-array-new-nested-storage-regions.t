// expect exit 0
// Empty length-check regions must leave stable cleanup and insertion identities.
int alive;
struct Element {
  Element() { ++alive; }
  ~Element() { --alive; }
};
struct Owner {
  Element* p;
  ~Owner() { delete[] p; }
};
Owner* make(int n) { return new Owner[n]{{new Element[n]}, {new Element[n]}}; }
int main() {
  Owner* p = make(2);
  if (alive != 4) return 1;
  delete[] p;
  return alive == 0 ? 0 : 2;
}
