// Forming `typename holder<T, D>::pointer` as an argument completes
// holder<T, D> while T and D are still parameters -- and the alias here
// discards that argument, so the completion happens purely to build something
// nobody keeps.  Such a completion is a shape: a member whose type is one of
// the parameters has a stand-in for its type, and a stand-in has no size.
// Asking for one turns a specialization that only had to exist into an
// incomplete-type error.
//
// Skipping that member's layout must not leak into a real one, so this also
// pins the size of a concrete specialization, whose members do have sizes.

template<class Type, class...>
using discard_keys = Type;

template<class T>
struct hash;

template<class T, class D>
struct holder
{
  typedef T *pointer;
  D deleter_;
};

template<class T, class D>
struct hash<discard_keys<holder<T, D>, typename holder<T, D>::pointer> >
{
  static const int selected = 1;
};

struct two_bytes { char a, b; };

int main()
{
  int status = 0;
  if (hash<holder<int, char> >::selected != 1) status = 1;
  // The concrete completion lays the member out for real.
  if (sizeof(holder<int, char>) != sizeof(char)) status = 2;
  if (sizeof(holder<int, two_bytes>) != sizeof(two_bytes)) status = 3;
  return status;
}
