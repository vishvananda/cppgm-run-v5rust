struct base {
  int tag;
  base(int v) : tag(v) {}
};

struct leaf : base {
  int value;
  leaf() : base(3), value(11) {}
  virtual ~leaf() {}
};

int read(base * root)
{
  leaf * p = static_cast<leaf *>(root);
  return p->value;
}

int main()
{
  leaf object;
  base * root = &object;
  return read(root) == 11 ? 0 : 1;
}
