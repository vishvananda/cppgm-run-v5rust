int depth;

struct Guard
{
  Guard() { ++depth; }
  ~Guard() { --depth; }
};

void raise(int value)
{
  throw value;
}

void exercise()
{
  Guard guard;
  try {
    raise(1);
  } catch(int) {
    if(depth == 1)
      raise(2);
  }
}

int main()
{
  try {
    exercise();
  } catch(int value) {
    return value != 2 || depth != 0;
  }
  return 1;
}
