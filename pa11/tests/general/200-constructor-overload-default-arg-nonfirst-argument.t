// VALIDATION: compile-pass
// N3485 focus: 13.3.1.3 [over.match.ctor], 8.3.6 [dcl.fct.default]

struct Alloc
{
  int marker;
};

Alloc default_alloc;

struct Item
{
  int marker;
  Item(int v) : marker(v) {}
};

struct Bucket
{
  int selected;

  Bucket(int count, const Alloc & alloc) : selected(1) {}

  Bucket(int count, const Item & item, const Alloc & alloc = default_alloc)
    : selected(item.marker)
  {
  }
};

int main()
{
  Item item(7);
  Bucket bucket(3, item);
  return bucket.selected == 7 ? 0 : 1;
}
