// N3485 5.3.4p6, 5.3.5p1, 6.4.2p2: discover the single non-explicit
// integral or object-pointer conversion before consuming its result.
// Reference results are read once, with their qualifiers; expect exit 0
int conversions;
int destroyed;
struct Extent {
  short n;
  operator volatile short&() { ++conversions; return n; }
};
struct State {
  State(int) {}
  ~State() { ++destroyed; }
  operator int() const { ++conversions; return 3; }
};
struct Object {
  ~Object() { ++destroyed; }
};
struct Owner {
  Object* p;
  operator Object* const&() const { ++conversions; return p; }
};
int main() {
  Extent extent = {3};
  switch (((extent))) {
  case 3: break;
  default: return 1;
  }
  int* values = new int[((extent))];
  values[2] = 7;
  if (values[2] != 7) return 2;
  delete[] values;
  switch (State state = 0) {
  case 3: if (destroyed != 0) return 3; break;
  default: return 4;
  }
  if (destroyed != 1) return 5;
  Owner single = {new Object};
  delete ((single));
  Owner array = {new Object[2]};
  delete[] ((array));
  return conversions == 5 && destroyed == 4 ? 0 : 6;
}
