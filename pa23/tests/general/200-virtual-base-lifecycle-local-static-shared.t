// A constructor's and a destructor's statics are one object for the
// complete-object entry and the base-subobject entry a class with a virtual
// base gives each (N3485 6.7/4).
int destroyed = 0;

struct shared
{
  int tag;
  shared() : tag(1) {}
};

struct counted : virtual shared
{
  int order;

  counted()
  {
    static int count = 0;
    count = count + 1;
    order = count;
  }

  ~counted()
  {
    static int gone = 0;
    gone = gone + 1;
    destroyed = gone;
  }
};

struct derived : counted
{
  derived() {}
};

int main()
{
  {
    counted whole;
    derived part;
    if (whole.order != 1) return 10;
    if (part.order != 2) return 11;
  }
  return destroyed == 2 ? 0 : 12;
}
