struct block {
  typedef block* pointer;
  void unlink() {}
  static void unlink(pointer pb) { pb->unlink(); }
};

int main()
{
  block b;
  block::unlink(&b);
  return 0;
}
