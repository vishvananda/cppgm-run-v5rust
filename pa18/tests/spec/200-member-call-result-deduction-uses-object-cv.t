// VALIDATION: compile-pass
// N3485 focus: 14.8.2.1 [temp.deduct.call]

template<class T>
int same_mutable(T &, T &)
{
  return 1;
}

struct Holder {
  int value;

  const int & get() const
  {
    return value;
  }

  int & get()
  {
    return value;
  }
};

int main()
{
  Holder holder = { 1 };
  int other = 2;
  return same_mutable(holder.get(), other) == 1 ? 0 : 1;
}
