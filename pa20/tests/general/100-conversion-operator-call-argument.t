// HHC-285
struct Target {
  Target() {}
};

struct Source {
  operator Target() const { return Target(); }
};

void sink(const Target &) {}

int main() {
  Source s;
  sink(s);
}
