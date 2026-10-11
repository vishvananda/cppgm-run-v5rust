// VALIDATION: compile-pass
// A template static constexpr class object retains its string-literal pointer
// through the emitted initialization path.

struct entry
{
  const char *name;

  constexpr entry(const char *value) : name(value)
  {
  }
};

template<class T>
struct table
{
  static constexpr entry row = entry("one");
};

template<class T>
constexpr entry table<T>::row;

int main()
{
  return table<int>::row.name[0] == 'o' ? 0 : 1;
}
