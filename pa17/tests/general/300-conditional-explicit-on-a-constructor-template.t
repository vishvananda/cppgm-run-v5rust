// `explicit(cond)` is not `explicit`.  A function template records its
// specifiers from the pattern, and taking the keyword alone there makes every
// conditional constructor explicit, which drops it from copy initialization.
// The condition can name the template's own parameters, so it cannot be
// answered when the pattern is recorded; the instantiation answers it with the
// arguments known.
//
// libc++ spells pair's converting constructor this way, so with the condition
// discarded nothing could convert between two pair specializations.

template<bool B>
struct pick
{
  static const bool implicit = B;
};

template<class T>
struct box
{
  T v_;

  box() : v_() {}

  // Implicit: the condition is false, so this takes part in copy
  // initialization.
  template<class U>
  explicit(!pick<true>::implicit) box(const box<U> &other) : v_(other.v_) {}
};

template<class T>
struct guarded
{
  T v_;

  guarded() : v_() {}

  // Explicit: the condition is true, so this must not take part in copy
  // initialization, and a direct-initialization still selects it.
  template<class U>
  explicit(!pick<false>::implicit) guarded(const guarded<U> &other)
    : v_(other.v_) {}
};

void take_box(box<long> b) { (void)b; }

int main()
{
  box<int> narrow;
  take_box(narrow);

  guarded<int> source;
  guarded<long> direct(source);

  return (direct.v_ == 0) ? 0 : 1;
}
