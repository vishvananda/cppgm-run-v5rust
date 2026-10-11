int g;
union Payload {
  int i;
  long long bits;
};

struct Sink {
  ~Sink() { g = 1; }
};

struct Wrapper {
  Payload payload;
  Sink sink;
};

int main() {
  Wrapper w;
  return 0;
}
