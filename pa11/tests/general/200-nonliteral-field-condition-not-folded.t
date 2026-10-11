// Field offsets also use CallSem integer metadata. They are not literal
// condition values and must still emit a runtime branch.

struct state
{
  bool fallback;
  int *value;

  bool read() const
  {
    if(value) {
      return *value != 0;
    }
    return fallback;
  }
};

int main()
{
  int one = 1;
  state empty = {false, 0};
  state full = {false, &one};
  return !empty.read() && full.read() ? 0 : 1;
}
