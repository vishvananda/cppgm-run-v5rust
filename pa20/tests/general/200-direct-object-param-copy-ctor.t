struct Base {
  int value;
};

struct Direct : Base {
  Direct(const Direct & other) : Base(other) {}
};

int take_direct(Direct direct)
{
  return direct.value;
}
