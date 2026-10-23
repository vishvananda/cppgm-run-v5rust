// expect exit 0
// A failed row destroys its prefix before completed preceding rows.
int order, failure;
struct Element {
  int value;
  Element(int n) : value(n) { if (n==failure) throw n; }
  ~Element() noexcept { order=order*10+value; }
};
int read(const Element (&array)[2][2], int n) { return array[1][1].value+n; }
int fail() { throw 5; }
struct Aggregate {
  int value;
  ~Aggregate() noexcept { order=order*10+value; }
};
int requests;
int next() { if (++requests==2) throw 6; return requests; }
int read_aggregates(const Aggregate (&array)[2]) { return array[0].value; }
int main() {
  failure=4;
  try {
    const Element (&array)[2][2]={{Element(1),Element(2)},{Element(3),Element(4)}};
    return 1;
  } catch (int n) { if (n!=4 || order!=321) return 2; }
  order=0;
  try {
    read({{Element(1),Element(2)},{Element(3),Element(4)}},0);
    return 3;
  } catch (int n) { if (n!=4 || order!=321) return 4; }
  failure=0;
  order=0;
  try {
    read({{Element(1),Element(2)},{Element(3),Element(4)}},fail());
    return 5;
  } catch (int n) { if (n!=5 || order!=4321) return 6; }
  order=0;
  try {
    const Aggregate (&array)[2]={{next()},{next()}};
    return 7;
  } catch (int n) { if (n!=6 || order!=1) return 8; }
  requests=order=0;
  try { read_aggregates({{next()},{next()}}); return 9; }
  catch (int n) { if (n!=6 || order!=1) return 10; }
  return 0;
}
