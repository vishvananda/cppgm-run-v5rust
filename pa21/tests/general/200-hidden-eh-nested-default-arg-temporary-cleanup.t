// VALIDATION: compile-pass
// N3485 focus: 12.2 [class.temporary]

int observed = 0;

struct Guard {
  Guard() {}
  ~Guard() {}
};

struct Alloc {
  int value;

  Alloc() : value(4) {}
  ~Alloc() {}
};

struct Target {
  int value;

  Target(const char *, const Alloc &alloc = Alloc()) : value(alloc.value) {}
  ~Target() {}
};

void record(const Guard &, int value)
{
  observed = value;
}

int main()
{
  record(Guard(), Target("x").value);
  return observed == 4 ? 0 : 1;
}
