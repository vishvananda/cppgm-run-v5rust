// VALIDATION: compile-pass
// N3485 focus: 14.5.1 [temp.class], 9.3 [class.mfct]

template<class Original>
struct owner
{
  int size();
};

template<class Renamed>
int owner<Renamed>::size()
{
  return sizeof(Renamed);
}

int main()
{
  owner<int> instance;
  return instance.size() == sizeof(int) ? 0 : 1;
}
