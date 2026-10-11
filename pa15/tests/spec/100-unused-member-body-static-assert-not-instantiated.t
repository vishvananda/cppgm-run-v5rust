// VALIDATION: compile-pass
// N3485 focus: 14.7.1 [temp.inst] implicit instantiation of class templates

struct Incomplete;

template<class T>
struct box
{
  int unused()
  {
    static_assert(sizeof(T) > 0, "unused member body is not instantiated");
    return 0;
  }

  int later()
  {
    return 7;
  }
};

int main()
{
  box<Incomplete> value;
  return sizeof(box<Incomplete>) == 1 && value.later() == 7 ? 0 : 1;
}
