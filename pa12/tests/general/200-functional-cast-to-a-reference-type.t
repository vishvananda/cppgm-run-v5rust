// N3485 5.2.3/1: `T(expr)` with one operand is the cast `(T)expr`.  When T is
// a reference type that binds the operand; nothing is constructed.  A class
// type reached through a reference still looks like a class object, so routing
// on that alone sends a reference cast to the class path, which initializes a
// fresh object from its own operand.
//
// libc++'s sort passes its comparator on as `_Comp_ref(__comp)`, whose
// `_Comp_ref` is a reference typedef.

struct pair_of
{
  int first;
  int second;
};

typedef pair_of &lvalue_ref;
typedef pair_of &&rvalue_ref;

struct counted
{
  int v;
  counted(int x) : v(x) {}
};

int main()
{
  pair_of made;
  made.first = 1;
  made.second = 2;

  // Binds: the reference names the same object, so a write through it shows.
  lvalue_ref bound = lvalue_ref(made);
  bound.first = 7;

  // The cast notation has always bound; the two spellings must agree.
  lvalue_ref also = (lvalue_ref)made;
  also.second = 8;

  pair_of moved = rvalue_ref(made);

  // A non-reference cast still constructs.
  counted built = counted(3);

  int status = 0;
  if (made.first != 7) status = 1;
  if (made.second != 8) status = 2;
  if (moved.first != 7 || moved.second != 8) status = 3;
  if (built.v != 3) status = 4;
  return status;
}
