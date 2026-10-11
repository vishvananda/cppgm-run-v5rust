struct downcast_root {
  int tag;
  downcast_root(int v) : tag(v) {}
};

struct downcast_leaf : downcast_root {
  int value;
  downcast_leaf() : downcast_root(3), value(11) {}
  virtual ~downcast_leaf() {}
};

int read(downcast_root & root)
{
  downcast_leaf & leaf = static_cast<downcast_leaf &>(root);
  return leaf.value;
}

int main()
{
  downcast_leaf leaf;
  return read(leaf) == 11 ? 0 : 1;
}
