// A function body the compiler lowers more than once declares each static
// variable once (N3485 6.7/4): a constructor's complete-object and
// base-subobject entries share their statics, and so does a member
// function another member calls before its definition.
struct counted
{
  int order;

  counted()
  {
    static int count = 0;
    count = count + 1;
    order = count;
  }
};

struct derived : counted
{
  derived() {}
};

struct walker
{
  int up(int depth) const;
  int down(int depth) const;
};

int walker::down(int depth) const { return up(depth + 1); }

int walker::up(int depth) const
{
  static int first = depth + 5;
  if (depth > 2) return first;
  return down(depth);
}

int main()
{
  counted whole;
  derived part;
  if (whole.order != 1) return 10;
  if (part.order != 2) return 11;
  walker w;
  return w.up(0) == 5 ? 0 : 13;
}
