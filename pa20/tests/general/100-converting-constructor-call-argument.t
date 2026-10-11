// HHC-285
struct Source {};
struct Target {
  Target() {}
  Target(const Source &) {}
};

void sink(const Target &) {}

int main() {
  Source s;
  sink(s);
}
