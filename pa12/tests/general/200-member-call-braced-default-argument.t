struct storage_ptr {
  int tag;
  storage_ptr() : tag(7) {}
};

int seen = 0;

struct parser {
  parser()
  {
    reset();
  }

  void reset(storage_ptr sp = {})
  {
    seen = sp.tag;
  }
};

int main()
{
  parser p;
  return seen == 7 ? 0 : 1;
}
