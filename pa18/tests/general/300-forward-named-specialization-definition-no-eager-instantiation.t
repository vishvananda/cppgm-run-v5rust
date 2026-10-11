// VALIDATION: compile-pass
// Naming an incomplete class-template specialization through a pointer does
// not instantiate its body when the primary definition later appears.

template<class T>
struct holder;

typedef holder<int> * dormant_pointer;

template<class T>
struct holder
{
  typename T::missing member;
};

int main()
{
  dormant_pointer pointer = 0;
  return pointer == 0 ? 0 : 1;
}
