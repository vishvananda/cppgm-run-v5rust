// VALIDATION: compile-pass
// A specialization named while the primary is incomplete still selects the
// matching partial specialization when completeness is later required.

template<class T>
struct pick;

typedef pick<char *> named_early;

template<class T>
struct pick
{
  static const int value = 1;
};

template<class T>
struct pick<T *>
{
  static const int value = 2;
};

int main()
{
  return named_early::value == 2 ? 0 : 1;
}
