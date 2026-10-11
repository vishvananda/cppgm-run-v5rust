// VALIDATION: compile-pass
// N3485 focus: 14.7.2 [temp.explicit]

template<class T>
struct box {};

template<class T>
int operator+(box<T>, box<T>);

extern template int operator+<int>(box<int>, box<int>);

int main()
{
  return 0;
}
