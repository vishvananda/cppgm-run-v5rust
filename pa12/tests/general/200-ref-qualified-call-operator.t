struct F {
  int operator()() & { return 3; }
  int operator()() && { return 4; }
};

int main() {
  F f;
  return f() + F()() - 7;
}
