// VALIDATION: compile-pass

struct State {
  int value;
};

struct Stream {
  void state(int) {}
  State state() const;
};

int main()
{
  Stream stream;
  stream.state(1);
  return 0;
}
