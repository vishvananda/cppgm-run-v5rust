// VALIDATION: compile-pass

struct Box {
  int value;

  Box() : value(0) {}
  Box(int v) : value(v) {}

  Box &operator=(const Box &other)
  {
    value = other.value;
    return *this;
  }
};

int main()
{
  Box box;
  box = 7;
  return box.value == 7 ? 0 : 1;
}
