namespace std {
using nullptr_t = decltype(nullptr);
}

struct storage_ptr {
  int tag;
  storage_ptr() : tag(7) {}
};

struct value {
  int tag;
  value(std::nullptr_t, storage_ptr sp = {}) : tag(sp.tag) {}
};

value parse(bool error)
{
  if(error)
    return nullptr;
  return value(nullptr);
}

int main()
{
  value v = parse(true);
  return v.tag == 7 ? 0 : 1;
}
