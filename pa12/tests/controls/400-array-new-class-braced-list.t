// Array new of class elements from a braced-init-list: each clause
// copy-initializes the next element (8.5.1), converting constructors and
// aggregate clauses included, and the elements past the list are
// value-initialized (8.5.1p7), here by the default constructor; delete[]
// destroys them in reverse order. A constant count the list fills needs no
// default constructor.
int events[32];
int event_count = 0;

void note(int event) {
  if (event_count < 32) events[event_count++] = event;
}

struct Counted {
  int id;
  Counted() : id(90 + event_count) { note(id); }
  Counted(int v) : id(v) { note(v); }
  ~Counted() { note(-id); }
};

struct NoDefault {
  int value;
  NoDefault(int v) : value(v) {}
};

struct Pair {
  int first;
  long second;
};

int count(int value) { return value; }

int main() {
  Counted* counted = new Counted[count(4)]{1, Counted(2)};
  delete[] counted;
  static const int expected[] = { 1, 2, 92, 93, -93, -92, -2, -1 };
  if (event_count != 8) return 1;
  for (int i = 0; i < 8; ++i)
    if (events[i] != expected[i]) return 2 + i;
  NoDefault* exact = new NoDefault[2]{3, 4};
  if (exact[0].value != 3 || exact[1].value != 4) return 20;
  delete[] exact;
  Pair* pairs = new Pair[count(3)]{{5, 6}, {7}};
  if (pairs[0].second != 6 || pairs[1].first != 7 || pairs[1].second != 0 ||
    pairs[2].first != 0 || pairs[2].second != 0) return 21;
  delete[] pairs;
  return 0;
}
