// VALIDATION: compile-pass
// N3485 focus: 9.2 [class.mem], 9.5 [class.union]

struct Store {
  union {
    int value;
    char bytes[4];
  };

  int get()
  {
    return value;
  }
};

int main()
{
  Store store = { { 7 } };
  return store.get() == 7 ? 0 : 1;
}
